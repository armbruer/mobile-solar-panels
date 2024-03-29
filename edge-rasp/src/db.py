# db code is based on https://github.com/dominicmason555/mqtt_to_timescale/blob/master/mqtt_to_timescale.py
import asyncio
import datetime

import asyncpg
import logging
import pandas as pd

from typing import List
from model import DataPoint, Config

QUERY_CREATE_SENSORS = """
CREATE TABLE IF NOT EXISTS sensor (
    time TIMESTAMPTZ NOT NULL,
    device_id INTEGER NOT NULL,
    temperature REAL NULL,
    photoresistor INTEGER NULL,
    power INTEGER NULL,
    PRIMARY KEY ("time")
);
"""

QUERY_INSERT_SENSORS = """
INSERT INTO sensor (time, device_id, temperature, photoresistor, power) VALUES ($1, $2, $3, $4, $5);
"""

QUERY_GET_SENSORS = """
SELECT * FROM sensor
WHERE time >= $1;
"""


async def run_db(pool: asyncpg.Pool, received_data_points: asyncio.Queue):
    async with pool.acquire() as conn:
        async with conn.transaction():
            await setup_table(conn)

    await worker(pool, received_data_points)


async def worker(pool: asyncpg.Pool, received_data_points: asyncio.Queue):
    while True:
        dps: List[DataPoint] = await received_data_points.get()

        await store_datapoints(pool, dps)


async def connect(conf: Config):
    logging.info("Connecting to database")
    try:
        pool = await asyncpg.create_pool(user=conf.db.user,
                                         password=conf.db.password,
                                         host=conf.db.host,
                                         port=conf.db.port,
                                         database=conf.db.database,
                                         min_size=2)
    except asyncpg.InterfaceError as ex:
        logging.critical("Failed to connect to database")
        print(ex)
        return

    logging.info("Connected to database")
    return pool


async def setup_table(conn: asyncpg.connection):
    logging.info("Initialising sensors datapoint table")
    await conn.execute(QUERY_CREATE_SENSORS)


async def store_datapoints(pool: asyncpg.Pool, datapoints: List[DataPoint]):
    try:
        async with pool.acquire() as conn:
            async with conn.transaction():
                await parse_insert(datapoints, conn)
    except asyncpg.InterfaceError as ex:
        logging.critical(f"DB connection failure while trying to store data: {ex}")


async def parse_insert(datapoints: List[DataPoint], conn: asyncpg.connection):
    for dp in datapoints:
        try:
            await conn.execute(QUERY_INSERT_SENSORS, dp.timestamp, dp.device_id,
                               dp.temperature, dp.photoresistor, dp.power)
        except asyncpg.InterfaceError as ex:
            logging.error(f"Sensors DB connection failure during storing data: {ex}")


async def get_datapoints(pool: asyncpg.Pool, start_time: datetime.datetime) -> pd.DataFrame:
    try:
        async with pool.acquire() as conn:
            async with conn.transaction():
                stmt = await conn.prepare(QUERY_GET_SENSORS)
                columns = [a.name for a in stmt.get_attributes()]
                data = await stmt.fetch(start_time)
                return pd.DataFrame(data, columns=columns)

    except asyncpg.InterfaceError as ex:
        logging.critical(f"DB connection failure while trying to retrieve data: {ex}")
        return pd.DataFrame()
