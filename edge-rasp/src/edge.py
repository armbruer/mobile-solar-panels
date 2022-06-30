import datetime
import enum
import logging

import asyncio
import threading

from aiohttp import web

import toml
from typing import List, Optional

import aiocoap.resource as resource
import aiocoap
import aiocoap.numbers.codes

import struct

from aiohttp.web_request import Request
from asyncio_mqtt import Client, MqttError
from pydantic import BaseModel, ValidationError
import suncalc

logging.basicConfig(level=logging.DEBUG)


class CommandTypes(enum.Enum):
    Nop = 0
    Location = 1
    LightTracking = 2
    Stop = 3


class CommandState:
    command: CommandTypes
    latitude: float
    longitude: float
    local_timezone: datetime.timezone

    def set_location_command_data(self, local_timezone, latitude, longitude):
        self.local_timezone = local_timezone
        self.latitude = latitude
        self.longitude = longitude


class Command:
    command: CommandTypes
    azimuth: float
    altitude: float

    def serialize(self) -> bytes:
        if self.command == CommandTypes.Location:
            return self.command.value.to_bytes(byteorder='little', signed=False, length=1) \
                   + struct.pack('<f', self.azimuth) + struct.pack('<f', self.altitude)
        else:
            return self.command.value.to_bytes(byteorder='little', signed=False, length=1)


class ConfigBroker(BaseModel):
    host: str
    port: int
    client_id: str


class Config(BaseModel):
    broker: ConfigBroker


class CommandResource(resource.Resource):
    command_state_lock: threading.Lock
    command_state: CommandState

    def __init__(self, command_state, command_state_lock):
        super().__init__()
        self.command_state = command_state
        self.command_state_lock = command_state_lock

        self.handle = None

    def get_link_description(self):
        # Publish additional data in .well-known/core
        return dict(**super().get_link_description(), title="Command pull resource.")

    async def render_get(self, request):
        self.command_state_lock.acquire()
        command_state = self.command_state
        self.command_state_lock.release()

        command = Command()
        command.command = command_state.command
        if command_state.command == CommandTypes.Location:
            local_time = datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc).astimezone(
                command_state.local_timezone)
            sun_loc = suncalc.get_position(local_time, lng=command_state.longitude,
                                           lat=command_state.latitude)
            command.azimuth = sun_loc["azimuth"]
            command.altitude = sun_loc["altitude"]

        return aiocoap.Message(payload=command.serialize())


class DataPoint:
    timestamp: datetime.datetime
    temperature: float
    photoresistor: int
    infrared: int
    voltage: int
    current: int
    power: int

    def __init__(self, timestamp, temperature, photoresistor, infrared, voltage, current, power):
        self.timestamp = timestamp
        self.temperature = temperature
        self.photoresistor = photoresistor
        self.infrared = infrared
        self.voltage = voltage
        self.current = current
        self.power = power

    def __str__(self):
        return self.timestamp.isoformat() + " " + str(self.temperature) + " " + str(self.photoresistor) + " " \
               + str(self.infrared) + " " + str(self.voltage) + " " + str(self.current) + " " + str(self.power)


class SensorData(resource.Resource):
    received_data_points: asyncio.Queue

    def __init__(self, received_data_points):
        super().__init__()
        self.received_data_points = received_data_points

    def get_link_description(self):
        # Publish additional data in .well-known/core
        return dict(**super().get_link_description(), title="Sensor data upload resource.")

    async def render_get(self, request):
        return aiocoap.Message(payload=b"some response payload")

    async def render_post(self, request):
        logging.debug(f"POST received payload: {request.payload}")

        edge_current_time = datetime.datetime.utcnow()

        payload: bytes = request.payload
        length_size = 4
        if len(payload) < length_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST, payload=b"Minimum packet size is 4")

        length = int.from_bytes(payload[0:4], byteorder='little', signed=False)

        # timestamp + 4 * 6 (temperature, photoresistor, ir sensor, voltage, current, power)
        all_fields_size = 8 + 4 * 6
        client_current_time_size = 8

        expected_packet_size = length_size + client_current_time_size + all_fields_size * length
        if len(payload) != expected_packet_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST,
                                   payload=b"Expected packet size: " + str(expected_packet_size).encode())

        client_current_time = int.from_bytes(payload[4:12], byteorder='little', signed=False)
        client_current_time = datetime.datetime.utcfromtimestamp(client_current_time)

        data = []

        index = length_size + client_current_time_size
        while index < len(payload):
            timestamp = int.from_bytes(payload[index:index + 8], byteorder='little', signed=False)
            timestamp = datetime.datetime.utcfromtimestamp(timestamp)
            # Time that passed since this datapoint was generated
            time_passed = client_current_time - timestamp
            timestamp = edge_current_time - time_passed
            index += 8

            temperature = struct.unpack('<f', payload[index:index + 4])[0]
            index += 4
            photoresistor = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
            index += 4
            infrared = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
            index += 4
            voltage = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
            index += 4
            current = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
            index += 4
            power = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
            index += 4

            data_point = DataPoint(timestamp=timestamp, temperature=temperature, photoresistor=photoresistor,
                                   infrared=infrared, voltage=voltage, current=current, power=power)
            data.append(data_point)

        assert index == len(payload)

        logging.debug("Sending datapoints to message queue...")
        await self.received_data_points.put(data)

        return aiocoap.Message(code=aiocoap.numbers.codes.Code.CHANGED, payload=b"ok")


async def worker(client: Client, received_data_points: asyncio.Queue):
    logging.debug("Started worker loop")
    while True:
        datapoints: List[DataPoint] = await received_data_points.get()
        message = ';'.join(map(str, datapoints))

        try:
            logging.debug("Publishing sensor data")
            await client.publish("sensors", payload=message.encode())
        except MqttError as ex:
            logging.error(ex)


def update_command(app, command_type: CommandTypes, timeoffset=None, latitude=None, longitude=None):
    command_state = app['command_state']
    command_state_lock: threading.Lock = app['command_state_lock']
    command_state_lock.acquire()
    command_state.command = command_type
    if latitude is not None and longitude is not None:
        local_timezone = datetime.timezone(offset=datetime.timedelta(minutes=timeoffset))
        command_state.set_location_command_data(local_timezone, latitude, longitude)
    command_state_lock.release()


async def location(request: Request):
    data = await request.json()
    update_command(request.app, CommandTypes.Location, data['timeoffset'], data['latitude'], data['longitude'])
    return web.Response()


async def light_tracking(request: Request):
    update_command(request.app, CommandTypes.LightTracking)
    return web.Response()


async def stop(request: Request):
    update_command(request.app, CommandTypes.Stop)
    return web.Response()


async def control(_request: Request):
    return web.FileResponse("control.html")


async def generate_data(received_data_points: asyncio.Queue):
    logging.warning("Creating mock values")
    while True:
        dp = DataPoint(datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc), 8651.1876, 588, 9911)
        await received_data_points.put([dp])
        logging.info("put some data")
        await asyncio.sleep(1)


def run_http_server(command_state: CommandState, command_state_lock: threading.Lock):
    app = web.Application()
    app['command_state'] = command_state
    app['command_state_lock'] = command_state_lock
    app.add_routes([web.post('/api/v1/location', location)])
    app.add_routes([web.post('/api/v1/light_tracking', light_tracking)])
    app.add_routes([web.post('/api/v1/stop', stop)])
    app.add_routes([web.get('/', control)])
    web.run_app(app)


async def run_coap_mqtt(conf: Config, command_state: CommandState, command_state_lock: threading.Lock):
    received_data_points = asyncio.Queue()

    # Resource tree creation
    root = resource.Site()
    root.add_resource(['.well-known', 'core'],
                      resource.WKCResource(root.get_resources_as_linkheader))
    root.add_resource(['command'], CommandResource(command_state, command_state_lock))
    root.add_resource(['sensor', 'data'], SensorData(received_data_points))

    logging.info("Creating CoAP server context")
    await aiocoap.Context.create_server_context(root)

    try:
        logging.info("Connecting to MQTT broker")
        async with Client(conf.broker.host, conf.broker.port, client_id=conf.broker.client_id) as client:
            logging.info("Connected to MQTT broker")
            await asyncio.gather(asyncio.get_running_loop().create_future(), worker(client, received_data_points))
    except MqttError as ex:
        logging.critical("MQTT Error")
        print(ex)


def main():
    command_state = CommandState()
    command_state_lock = threading.Lock()

    thread_http_server = threading.Thread(target=run_http_server, args=(command_state, command_state_lock))
    thread_http_server.start()

    try:
        config_dict = toml.load("config.toml")
        config = Config.parse_obj(config_dict)
        asyncio.run(run_coap_mqtt(config, command_state, command_state_lock))
    except ValidationError as e:
        logging.critical("Failed to load config file")
        print(e)

    thread_http_server.join()


if __name__ == "__main__":
    main()
