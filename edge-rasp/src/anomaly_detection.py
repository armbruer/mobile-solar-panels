import asyncio
import logging
import db
import asyncpg
import pandas as pd
import datetime

from sklearn.cluster import DBSCAN
from sklearn.preprocessing import StandardScaler
from model import Config

ANOMALY_DETECTION_INTERVAL = datetime.timedelta(minutes=10).total_seconds()


async def run_anomaly_detection(pool: asyncpg.Pool, conf: Config):
    await worker(pool)


async def worker(pool: asyncpg.Pool):
    while True:
        await asyncio.sleep(ANOMALY_DETECTION_INTERVAL)

        df: pd.DataFrame = await db.get_datapoints(pool)
        length = len(df.index)
        if length == 0:
            logging.warning("No data for anomaly detection available")
        elif length <= 20:
            logging.warning("Not enough data for anomaly detection available")

        outliers_df = await run_dbscan(df)


async def run_dbscan(df: pd.DataFrame):
    data = df[["temperature", "power", "photoresistor"]]
    X = StandardScaler().fit(data)
    model: DBSCAN = DBSCAN(eps=0.5, min_samples=5).fit(X)

    # not sure if this works
    outliers_df = data[model.labels_ == -1]
    return outliers_df
