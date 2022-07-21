import asyncio
import logging
import os

import db
import asyncpg
import pandas as pd
import datetime
import aiosmtplib

from sklearn.cluster import DBSCAN
from sklearn.preprocessing import StandardScaler
from model import Config
from email.message import EmailMessage

MIN_DATAPOINTS = 20
ANOMALY_DETECTION_INTERVAL = int(datetime.timedelta(minutes=int(os.environ["ANOMALY_DETECTION_INTERVAL_MINUTES"])).total_seconds())
REPORT_INTERVAL = int(datetime.timedelta(minutes=int(os.environ["REPORT_INTERVAL_MINUTES"])).total_seconds())


async def run_anomaly_detection(pool: asyncpg.Pool, conf: Config):
    if not (REPORT_INTERVAL % ANOMALY_DETECTION_INTERVAL == 0):
        logging.error(f"REPORT_INTERVAL '{REPORT_INTERVAL}' must be a multiple of ANOMALY_DETECTION_INTERVAL"
                      f"'{ANOMALY_DETECTION_INTERVAL}'")

    await worker(pool, conf)


async def worker(pool: asyncpg.Pool, conf: Config):
    outliers_dfs = []
    # Due to REPORT_INTERVAL % ANOMALY_DETECTION_INTERVAL == 0 this does not remove decimal digits
    rounds = int(REPORT_INTERVAL / ANOMALY_DETECTION_INTERVAL)
    curr_round = 1

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

        if curr_round < rounds:
            outliers_dfs.append(await run_dbscan(df))
            curr_round += 1
        else:
            curr_round = 0
            await send_mail(conf, pd.concat(outliers_dfs))


async def send_mail(conf: Config, outliers_df: pd.DataFrame):
    device_ids = outliers_df['device_id'].unique()
    device_ids.sort()
    devices = ', '.join(map(str, device_ids))

    outliers_dfs = [y for _, y in pd.groupby('device_id', as_index=False)]
    anomalies = []
    for df in outliers_dfs:
        device_id = str(df.head(1)['device_id'])
        anomaly_datetimes = ', '.join(map(lambda x: x.isoformat(),
                                          outliers_df.sort_values(by=['time', 'device_id'])['time'].to_list()))
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

        await aiosmtplib.send(message, conf.anomaly_detection.smtp.email_sender, email_receiver,
                              conf.anomaly_detection.smtp.host, conf.anomaly_detection.smtp.port,
                              conf.anomaly_detection.smtp.user, conf.anomaly_detection.smtp.password,
                              use_tls=True)


async def run_dbscan(df: pd.DataFrame):
    data = df[["temperature", "power", "photoresistor"]]
    model: DBSCAN = DBSCAN(eps=0.5, min_samples=5).fit(data)

    # Use "df" that all columns are present
    outliers_df = df[model.labels_ == -1]
    return outliers_df
