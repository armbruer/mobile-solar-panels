import asyncio
import logging
import os

import db
import asyncpg
import pandas as pd
import datetime
import mail

from sklearn.cluster import DBSCAN
from model import Config

MIN_DATAPOINTS = 20
ANOMALY_DETECTION_INTERVAL = int(datetime.timedelta(
    minutes=int(os.environ["ANOMALY_DETECTION_INTERVAL_MINUTES"])).total_seconds())


async def run_anomaly_detection(pool: asyncpg.Pool, conf: Config):
    await worker(pool, conf)


async def worker(pool: asyncpg.Pool, conf: Config):
    while True:
        await asyncio.sleep(ANOMALY_DETECTION_INTERVAL)

        df: pd.DataFrame = await db.get_datapoints(pool)
        length = len(df.index)
        if length == 0:
            logging.warning("No data for anomaly detection available")
            continue
        elif length <= MIN_DATAPOINTS:
            logging.warning("Not enough data for anomaly detection available")
            continue

        await mail.send_mail(conf, await run_dbscan(df))


async def run_dbscan(df: pd.DataFrame):
    data = df[["temperature", "power", "photoresistor"]]
    model: DBSCAN = DBSCAN(eps=8, min_samples=6).fit(data)

    # Use "df" that all columns are present
    outliers_df = df[model.labels_ == -1]

    return outliers_df
