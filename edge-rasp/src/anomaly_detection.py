import logging
import os
import db
import asyncpg
import pandas as pd
import datetime
import mail
import asyncio

from sklearn.preprocessing import StandardScaler
from sklearn.cluster import DBSCAN
from model import Config

MIN_DATAPOINTS = 20
ANOMALY_DETECTION_INTERVAL = int(datetime.timedelta(
    seconds=int(os.environ.get("ANOMALY_DETECTION_INTERVAL_SECONDS", 30))).total_seconds())


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


async def run_threshold(df: pd.DataFrame):
    # thresholds are based on the data we collected with ~20-30% margin
    return pd.concat([df[df["power"] < 60], df[df["power"] > 1000],
                      df[df["temperature"] > 100.0], df[df["temperature"] < -30.0],
                      df[df["photoresistor"] < 90], df[df["photoresistor"] > 300]])


async def run_dbscan(df: pd.DataFrame):
    data = df[["temperature", "power", "photoresistor"]]
    scaler = StandardScaler()
    scaler = scaler.fit_transform(data)
    model: DBSCAN = DBSCAN(eps=0.245, min_samples=6).fit(scaler)

    # Use "df" that all columns are present
    outliers_df = df[model.labels_ == -1]

    return outliers_df


async def test():
    dataset = pd.read_csv(filepath_or_buffer="../anomaly-data/data.csv", sep=";", usecols=[0, 2, 3, 4])
    outliers = await run_dbscan(dataset)
    print(outliers.shape)
    print(outliers)


if __name__ == '__main__':
    asyncio.run(test())
