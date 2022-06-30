import datetime
import logging

import asyncio

from aiohttp import web

import toml
from typing import List

import aiocoap.resource as resource
import aiocoap
import aiocoap.numbers.codes

import struct

from aiohttp.web_request import Request
from asyncio_mqtt import Client, MqttError
from pydantic import BaseModel, ValidationError
from suncalc import get_position

logging.basicConfig(level=logging.DEBUG)


class ConfigBroker(BaseModel):
    host: str
    port: int
    client_id: str


class Config(BaseModel):
    broker: ConfigBroker


class CommandResource(resource.Resource):  # maybe use ObservableResource
    command_queue: asyncio.Queue
    """Example resource that can be observed. The `notify` method keeps
    scheduling itself, and calles `update_state` to trigger sending
    notifications."""

    def __init__(self, command_queue):
        super().__init__()
        self.command_queue = command_queue

        self.handle = None

    # def notify(self):
    #     self.updated_state()
    #     self.reschedule()

    # def reschedule(self):
    #     self.handle = asyncio.get_event_loop().call_later(5, self.notify)

    # def update_observation_count(self, count):
    #     if count and self.handle is None:
    #         logging.debug("Starting the clock")
    #         self.reschedule()
    #     if count == 0 and self.handle:
    #         logging.debug("Stopping the clock")
    #         self.handle.cancel()
    #         self.handle = None

    def get_link_description(self):
        # Publish additional data in .well-known/core
        return dict(**super().get_link_description(), title="Command download resource.")

    async def render_get(self, request):
        # or use await and get() to wait for next command
        payload = self.command_queue.get_nowait()
        return aiocoap.Message(payload=payload)


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
        return self.timestamp.isoformat() + " " + str(self.temperature) + " " + str(self.photoresistor) + " "\
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
            timestamp = int.from_bytes(payload[index:index+8], byteorder='little', signed=False)
            timestamp = datetime.datetime.utcfromtimestamp(timestamp)
            # Time that passed since this datapoint was generated
            time_passed = client_current_time - timestamp
            timestamp = edge_current_time - time_passed
            index += 8

            temperature = struct.unpack('<f', payload[index:index+4])[0]
            index += 4
            photoresistor = int.from_bytes(payload[index:index+4], byteorder='little', signed=True)
            index += 4
            infrared = int.from_bytes(payload[index:index+4], byteorder='little', signed=True)
            index += 4
            voltage = int.from_bytes(payload[index:index + 4], byteorder='little', signed=True)
            index += 4
            current = int.from_bytes(payload[index:index + 4], byteorder='little', signed=True)
            index += 4
            power = int.from_bytes(payload[index:index + 4], byteorder='little', signed=True)
            index += 4

            data_point = DataPoint(timestamp=timestamp, temperature=temperature, photoresistor=photoresistor,
                                   infrared=infrared, voltage=voltage, current=current, power=power)
            data.append(data_point)

        assert index == len(payload)

        logging.debug("Sending datapoints to message queue...")
        await self.received_data_points.put(data)

        # latitude = self.app['latitude']
        # longitude = self.app['longitude']
        # if len(latitude) > 0:
        #     position = get_position(datetime.datetime.now(),
        #                             latitude, data.longitude)
        #     return aiocoap.Message(code=aiocoap.numbers.codes.Code.CHANGED, payload=bytes(position))

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


async def location(request: Request):
    data = await request.json()
    print(data)
    position = get_position(datetime.datetime.now(),
                            data.latitude, data.longitude)
    position["command"] = "location"
    print(position)

    command_queue = request.app['command_queue']
    command_queue.put(position)
    return web.Response()


async def light_tracking(request: Request):
    command_queue = request.app['command_queue']
    command_queue.put({"command": "light_tracking"})
    return web.Response()


async def stop(request: Request):
    # request.app['latitude'] = ""
    # request.app['longitude'] = ""
    command_queue = request.app['command_queue']
    command_queue.put({"command": "stop"})
    return web.Response()


async def geolocation(request):
    return web.FileResponse("./geolocation.html")


async def generate_data(received_data_points: asyncio.Queue):
    logging.warning("Creating mock values")
    while True:
        dp = DataPoint(datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc), 8651.1876, 588, 9911)
        await received_data_points.put([dp])
        logging.info("put some data")
        await asyncio.sleep(1)


async def main(conf: Config):
    received_data_points = asyncio.Queue()
    command_queue = asyncio.Queue()

    app = web.Application()
    app['command_queue'] = command_queue
    app.add_routes([web.post('/api/v1/location', location)])
    app.add_routes([web.post('/api/v1/light_tracking', light_tracking)])
    app.add_routes([web.post('/api/v1/stop', stop)])
    app.add_routes([web.get('/', geolocation)])
    web.run_app(app)

    # Resource tree creation
    root = resource.Site()
    root.add_resource(['.well-known', 'core'],
                      resource.WKCResource(root.get_resources_as_linkheader))
    root.add_resource(['command'], CommandResource(command_queue))
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


if __name__ == "__main__":
    try:
        config_dict = toml.load("config.toml")
        config = Config.parse_obj(config_dict)
        asyncio.run(main(config))
    except ValidationError as e:
        logging.critical("Failed to load config file")
        print(e)
