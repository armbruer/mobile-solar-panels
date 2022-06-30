import asyncio
import logging
from typing import List

from asyncio_mqtt import Client, MqttError
from pydantic import BaseModel

from model import DataPoint


class ConfigBroker(BaseModel):
    host: str
    port: int
    client_id: str


class Config(BaseModel):
    broker: ConfigBroker


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


async def run_mqtt(conf: Config, received_data_points: asyncio.Queue):
    try:
        logging.info("Connecting to MQTT broker")
        async with Client(conf.broker.host, conf.broker.port, client_id=conf.broker.client_id) as client:
            logging.info("Connected to MQTT broker")
            await worker(client, received_data_points)
    except MqttError as ex:
        logging.critical("MQTT Error")
        print(ex)
