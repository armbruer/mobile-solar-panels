import datetime
import logging
import asyncpg
import struct

from typing import Any

QUERY_CREATE_SENSORS = """
CREATE TABLE IF NOT EXISTS sensor (
    time TIMESTAMPTZ NOT NULL,
    temperature REAL NULL,
    photoresistor INTEGER NULL,
    infrared INTEGER NULL,
    voltage INTEGER NULL,
    current INTEGER NULL,
    power INTEGER NULL,
    PRIMARY KEY ("time")
);
"""

QUERY_INSERT_SENSORS = """
INSERT INTO sensor (time, temperature, photoresistor, infrared, voltage, current, power) VALUES ($1, $2, $3, $4, $5, $6, $7)
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
    voltage: int
    current: int
    power: int

    def __init__(self, timestamp, temperature, photoresistor, infrared, voltage, current, power, **data: Any):
        super().__init__(**data)
        self.timestamp = timestamp
        self.temperature = temperature
        self.photoresistor = photoresistor
        self.infrared = infrared
        self.voltage = voltage
        self.current = current
        self.power = power

    @staticmethod
    def deserialize(payload: bytes):
        index = 0
        timestamp = int.from_bytes(payload[:8], byteorder='little', signed=False)
        timestamp = datetime.datetime.utcfromtimestamp(timestamp)
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
        return data_point


async def setup(conn: asyncpg.connection):
    logging.info("Initialising sensors datapoint table")
    await conn.execute(QUERY_CREATE_SENSORS)


async def parse_insert(payload: bytes, conn: asyncpg.connection):
    dp = DataPoint.deserialize(payload)

    try:
        await conn.execute(QUERY_INSERT_SENSORS, dp.timestamp, dp.temperature, dp.photoresistor, dp.infrared, dp.voltage, dp.current, dp.power)
    except asyncpg.InterfaceError as ex:
        logging.error("Sensors DB connection failure during storing data: " + str(ex))
