import logging
import asyncpg

from typing import Any

QUERY_CREATE_SENSORS = """
CREATE TABLE IF NOT EXISTS sensor (
    time TIMESTAMPTZ NOT NULL,
    temperature REAL NULL,
    photoresistor INTEGER NULL,
    infrared INTEGER NULL,
);
"""

QUERY_INSERT_SENSORS = """
INSERT INTO sensor (time, temperature, photoresistor, infrared) VALUES (NOW(), $1 $2, $3)
"""


class DataPoint():
    temperature: float
    photoresistor: int
    infrared: int

    def __init__(self, temperature, photoresistor, infrared, **data: Any):
        super().__init__(**data)
        self.temperature = temperature
        self.photoresistor = photoresistor
        self.infrared = infrared

    @staticmethod
    def from_str(datapoint: str):
        t, p, i = datapoint.split(" ")
        return DataPoint(t, p, i)


async def setup(conn: asyncpg.connection):
    logging.info("Initialising sensors datapoint table")
    await conn.execute(QUERY_CREATE_SENSORS)


async def parse_insert(payload: str, conn: asyncpg.connection):
    # TODO data validation on both sides of mqtt
    datapoints = map(DataPoint.from_str, payload.split(";"))

    for dp in datapoints:
        try:
            await conn.execute(QUERY_INSERT_SENSORS, dp.temperature, dp.photoresistor, dp.infrared)
        except asyncpg.InterfaceError as ex:
            logging.critical("Sensors DB connection failure")
            print(ex)
