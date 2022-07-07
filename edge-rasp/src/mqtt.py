import asyncio
import logging
import datetime
import os
from typing import List, Dict

from asyncio_mqtt import Client, MqttError
from pydantic import BaseModel

from model import DataPoint


class ConfigBroker(BaseModel):
    host: str
    port: int
    client_id: str


class Config(BaseModel):
    broker: ConfigBroker


DATA_COLLECTION_INTERVAL = datetime.timedelta(seconds=int(os.environ["DATA_COLLECTION_INTERVAL_SECONDS"]))


# splits the datapoints list into sublists according to their device id
def split_datapoints(datapoints: List[DataPoint]) -> Dict[int, List[DataPoint]]:
    datapoints_dict: Dict[int, List[DataPoint]] = {}

    for dp in datapoints:
        if dp.device_id not in datapoints_dict:
            datapoints_dict[dp.device_id] = [dp]
        else:
            datapoints_dict[dp.device_id].append(dp)

    return datapoints_dict


async def worker(client: Client, received_data_points: asyncio.Queue):
    logging.debug("Started worker loop")

    end_of_interval: Dict[int, datetime] = {}
    datapoints: Dict[int, List[DataPoint]] = {}

    while True:
        received_dps: List[DataPoint] = await received_data_points.get()
        received_dps: Dict[int, List[DataPoint]] = split_datapoints(received_dps)

        for device_id, dps in received_dps.items():
            list.sort(dps, key=lambda x: x.timestamp)
            if device_id not in end_of_interval:
                end_of_interval[device_id] = dps[0].timestamp + DATA_COLLECTION_INTERVAL

        for device_id, dps in received_dps.items():
            next_datapoints: List[DataPoint] = []
            for dp in dps:
                if dp.timestamp < end_of_interval[device_id]:
                    if device_id not in datapoints:
                        datapoints[device_id] = []
                    datapoints[device_id].append(dp)
                else:
                    next_datapoints.append(dp)

            # at least one datapoint is of the new interval: aggregate and send data
            if next_datapoints:
                res_dp = DataPoint.aggregate_datapoints(datapoints)
                datapoints[device_id] = next_datapoints
                end_of_interval[device_id] = next_datapoints[0].timestamp + DATA_COLLECTION_INTERVAL

                try:
                    logging.debug("Publishing sensor data")
                    await client.publish("sensors", payload=res_dp.serialize())
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
