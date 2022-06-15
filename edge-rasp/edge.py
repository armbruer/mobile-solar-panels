import datetime
import logging

import asyncio
import toml
from typing import List

import aiocoap.resource as resource
import aiocoap
import aiocoap.numbers.codes

import struct
from asyncio_mqtt import Client, MqttError
from pydantic import BaseModel, ValidationError

logging.basicConfig(level=logging.INFO)
logging.getLogger("coap-server").setLevel(logging.DEBUG)


class ConfigBroker(BaseModel):
    host: str
    port: int


class Config(BaseModel):
    broker: ConfigBroker


class TimeResource(resource.ObservableResource):
    """Example resource that can be observed. The `notify` method keeps
    scheduling itself, and calles `update_state` to trigger sending
    notifications."""

    def __init__(self):
        super().__init__()

        self.handle = None

    def notify(self):
        self.updated_state()
        self.reschedule()

    def reschedule(self):
        self.handle = asyncio.get_event_loop().call_later(5, self.notify)

    def update_observation_count(self, count):
        if count and self.handle is None:
            logging.debug("Starting the clock")
            self.reschedule()
        if count == 0 and self.handle:
            logging.debug("Stopping the clock")
            self.handle.cancel()
            self.handle = None

    async def render_get(self, request):
        payload = datetime.datetime.now(). \
            strftime("%Y-%m-%d %H:%M").encode('ascii')
        return aiocoap.Message(payload=payload)


class DataPoint:
    temperature: float
    photoresistor: int
    infrared: int

    def __init__(self, temperature, photoresistor, infrared):
        self.temperature = temperature
        self.photoresistor = photoresistor
        self.infrared = infrared

    def __str__(self):
        return str(self.temperature) + " " + str(self.photoresistor) + " " + str(self.infrared)


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

        payload: bytes = request.payload
        length_size = 4
        if len(payload) < length_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST, payload=b"Minimum packet size is 4")

        length = int.from_bytes(payload[0:4], byteorder='little', signed=False)

        all_fields_size = 4 * 3
        expected_packet_size = length_size + all_fields_size * length
        if len(payload) != expected_packet_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST,
                                   payload=b"Expected packet size: " + str(expected_packet_size).encode())

        data = []

        index = length_size
        while index < len(payload):
            temperature = struct.unpack('<f', payload[index:index+4])
            photoresistor = int.from_bytes(payload[index+4:index+8], byteorder='little', signed=True)
            infrared = int.from_bytes(payload[index+8:index+12], byteorder='little', signed=True)
            index += all_fields_size

            data_point = DataPoint(temperature=temperature, photoresistor=photoresistor, infrared=infrared)
            data.append(data_point)

        logging.debug("Sending datapoints to message queue...")
        await self.received_data_points.put(data)

        return aiocoap.Message(code=aiocoap.numbers.codes.Code.CHANGED, payload=b"")


async def worker(client: Client, received_data_points: asyncio.Queue):
    while True:
        datapoints: List[DataPoint] = await received_data_points.get()
        message = ';'.join(map(str, datapoints))

        try:
            await client.publish("sensors", payload=message.encode())
        except MqttError as ex:
            logging.critical("MQTT Error")
            print(ex)


async def main(conf: Config):
    received_data_points = asyncio.Queue()

    logging.info("Connecting to MQTT broker")

    # Resource tree creation
    root = resource.Site()

    root.add_resource(['.well-known', 'core'],
                      resource.WKCResource(root.get_resources_as_linkheader))
    root.add_resource(['time'], TimeResource())
    root.add_resource(['sensor', 'data'], SensorData(received_data_points))

    await aiocoap.Context.create_server_context(root)

    # Run forever
    # await asyncio.get_running_loop().create_future() TODO, still required?

    try:
        async with Client(conf.broker.host, conf.broker.port) as client:
            logging.info("Connected to MQTT broker")
            await asyncio.gather(worker(client, received_data_points), asyncio.get_running_loop().create_future())
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
