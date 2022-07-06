import asyncio
import logging
import datetime
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

    end_of_interval = None
    datapoints: List[DataPoint] = []

    while True:
        received_dps: List[DataPoint] = await received_data_points.get()
        list.sort(received_dps, key=lambda a, b: a.timestamp < b.timestamp)
        if not end_of_interval:
            end_of_interval = received_dps[0].timestamp + datetime.timedelta(minutes=10)

        next_datapoints: List[DataPoint] = []
        for rdp in received_dps:
            if rdp.timestamp < end_of_interval:
                datapoints.append(rdp)
            else:
                next_datapoints.append(rdp)

        # at least one datapoint is of the new interval: aggregate and send data
        if next_datapoints:
            dp = DataPoint.aggregate_datapoints(datapoints)
            datapoints = next_datapoints
            end_of_interval = next_datapoints[0].timestamp + datetime.timedelta(minutes=10)

            try:
                logging.debug("Publishing sensor data")
                await client.publish("sensors", payload=str(dp).encode())
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
