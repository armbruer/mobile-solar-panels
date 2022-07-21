import datetime
import logging
import asyncpg
import struct

from dataclasses import dataclass

QUERY_CREATE_SENSORS = """
CREATE TABLE IF NOT EXISTS sensor (
    time TIMESTAMPTZ NOT NULL,
    device_id INTEGER NOT NULL,
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
INSERT INTO sensor (time, device_id, temperature, photoresistor, infrared, voltage, current, power) VALUES ($1, $2, $3, $4, $5, $6, $7, $8);
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
@dataclass
class DataPoint:
    device_id: int
    timestamp: datetime.datetime
    temperature: float
    photoresistor: int
    infrared: int
    voltage: int
    current: int
    power: int

    @staticmethod
    def deserialize(payload: bytes):
        index = 0
        device_id = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
        index += 4
        timestamp = int.from_bytes(payload[index:index + 8], byteorder='little', signed=False)
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

        return DataPoint(device_id=device_id, timestamp=timestamp, temperature=temperature, photoresistor=photoresistor,
                         infrared=infrared, voltage=voltage, current=current, power=power)


async def setup(conn: asyncpg.connection):
    logging.info("Initialising sensors datapoint table")
    await conn.execute(QUERY_CREATE_SENSORS)


async def parse_insert(payload: bytes, conn: asyncpg.connection):
    dp = DataPoint.deserialize(payload)

    try:
        await conn.execute(QUERY_INSERT_SENSORS, dp.timestamp, dp.device_id, dp.temperature, dp.photoresistor,
                           dp.infrared, dp.voltage, dp.current, dp.power)
    except asyncpg.InterfaceError as ex:
        logging.error("Sensors DB connection failure during storing data: " + str(ex))
