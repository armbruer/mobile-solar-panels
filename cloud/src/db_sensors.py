import datetime
import logging
import asyncpg

from typing import Any

QUERY_CREATE_SENSORS = """
CREATE TABLE IF NOT EXISTS sensor (
    time TIMESTAMPTZ NOT NULL,
    temperature REAL NULL,
    photoresistor INTEGER NULL,
    infrared INTEGER NULL,
    PRIMARY KEY ("time")
);
"""

QUERY_INSERT_SENSORS = """
INSERT INTO sensor (time, temperature, photoresistor, infrared) VALUES ($1, $2, $3, $4)
"""


def auto_str(cls):
    def __str__(self):
        return '%s(%s)' % (
            type(self).__name__,
            ', '.join('%s=%s' % item for item in vars(self).items())
        )
    cls.__str__ = __str__
    return cls


@auto_str
class DataPoint:
    timestamp: datetime.datetime
    temperature: float
    photoresistor: int
    infrared: int

    def __init__(self, timestamp, temperature, photoresistor, infrared, **data: Any):
        super().__init__(**data)
        self.timestamp = timestamp
        self.temperature = temperature
        self.photoresistor = photoresistor
        self.infrared = infrared

    @staticmethod
    def from_str(datapoint: str):
        try:
            timetz, temp, photo, infra = datapoint.split(" ")
            return DataPoint(datetime.datetime.fromisoformat(timetz), float(temp), int(photo), int(infra))
        except ValueError as ex:
            logging.error("Failed to parse value: " + str(ex) + " Datapoint: " + str(datapoint))
            raise ex


async def setup(conn: asyncpg.connection):
    logging.info("Initialising sensors datapoint table")
    await conn.execute(QUERY_CREATE_SENSORS)


async def parse_insert(payload: bytes, conn: asyncpg.connection):
    # TODO data validation on both sides of mqtt
    datapoints = map(DataPoint.from_str, payload.decode().split(";"))

    for dp in datapoints:
        try:
            await conn.execute(QUERY_INSERT_SENSORS, dp.timestamp, dp.temperature, dp.photoresistor, dp.infrared)
        except asyncpg.InterfaceError as ex:
            logging.error("Sensors DB connection failure during storing data: " + str(ex))
