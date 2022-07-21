import asyncio
import logging
import os

import db
import asyncpg
import pandas as pd
import datetime
import aiosmtplib

from sklearn.cluster import DBSCAN
from model import Config
from email.message import EmailMessage

MIN_DATAPOINTS = 20
ANOMALY_DETECTION_INTERVAL = int(datetime.timedelta(minutes=int(os.environ["ANOMALY_DETECTION_INTERVAL_MINUTES"])).total_seconds())
# TODO: Remove if not needed
# REPORT_INTERVAL = int(datetime.timedelta(minutes=int(os.environ["REPORT_INTERVAL_MINUTES"])).total_seconds())


async def run_anomaly_detection(pool: asyncpg.Pool, conf: Config):
    # if not (REPORT_INTERVAL % ANOMALY_DETECTION_INTERVAL == 0):
    #     logging.error(f"REPORT_INTERVAL '{REPORT_INTERVAL}' must be a multiple of ANOMALY_DETECTION_INTERVAL"
    #                   f"'{ANOMALY_DETECTION_INTERVAL}'")

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

        await send_mail(conf, await run_dbscan(df))


async def send_mail(conf: Config, outliers_df: pd.DataFrame):
    device_ids = outliers_df['device_id'].unique()
    device_ids.sort()
    devices = ', '.join(map(str, device_ids))

    outliers_dfs = [y for _, y in outliers_df.groupby('device_id', as_index=False)]
    anomalies = []
    for df in outliers_dfs:
        device_id = str(df.head(1)['device_id'].values[0])
        anomaly_datetimes = ', '.join(map(lambda x: x.isoformat(),
                                          df['time'].sort_values()))
        anomalies.append(device_id + ': ' + anomaly_datetimes)

    outliers = '\n'.join(anomalies)

    content = f"""Hello,
we have detected anomalies in your solar plants. The following devices may be affected:

{devices}

{outliers}

Best regards,
Your Mobile Solar Panels Team
"""

    message = EmailMessage()
    message["From"] = conf.anomaly_detection.smtp.user
    message["Subject"] = "Report: Anomalies detected"
    message.set_content(content)

    for email_receiver in conf.anomaly_detection.email_receivers:
        message["To"] = email_receiver

        await aiosmtplib.send(message=message, sender=conf.anomaly_detection.smtp.email_sender, recipients=email_receiver,
                              hostname=conf.anomaly_detection.smtp.host, port=conf.anomaly_detection.smtp.port,
                              username=conf.anomaly_detection.smtp.user, password=conf.anomaly_detection.smtp.password,
                              use_tls=True)


async def run_dbscan(df: pd.DataFrame):
    data = df[["temperature", "power", "photoresistor"]]
    model: DBSCAN = DBSCAN(eps=0.5, min_samples=5).fit(data)

    # Use "df" that all columns are present
    outliers_df = df[model.labels_ == -1]

    # In a perfect world we would have different clusters for different weather conditions
    # But maybe (if we arent lucky) if one device is broken, it is just wrongly classified (less power generated due to an error -> misinterpreted as cloudy weather)
    # Normally the photoresistor value should prevent this as this sensor would still identify the correct weather (if it isnt broken as well) but ¯\_(°_°)_/¯
    # An additional check if the datapoint of each device for one timestamp are in the same cluster could prevent this (or at least minimize the occurance of this error)

    return outliers_df
