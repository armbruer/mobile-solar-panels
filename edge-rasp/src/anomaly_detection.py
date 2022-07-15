import asyncio
import logging

import db
import asyncpg
import pandas as pd
import datetime
import aiosmtplib

from sklearn.cluster import DBSCAN
from sklearn.preprocessing import StandardScaler
from model import Config
from email.message import EmailMessage

ANOMALY_DETECTION_INTERVAL = datetime.timedelta(minutes=10).total_seconds()


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
        elif length <= 20:
            logging.warning("Not enough data for anomaly detection available")
            continue

        outliers_df = await run_dbscan(df)
        await send_mail(conf, outliers_df)


async def send_mail(conf: Config, outliers_df: pd.DataFrame):
    devices = ', '.join(map(str, set(outliers_df['device_id'].tolist())))

    # TODO improve message content
    content = f"""Hello,
we have detected anomalies in your solar plants. The following devices may be affected:

{devices}

Best regards,
Your Mobile Solar Panels Team
"""

    message = EmailMessage()
    message["From"] = conf.ad.smtp.user
    message["Subject"] = "Report: Anomalies detected"
    message.set_content(content)

    for email_receiver in conf.ad.email_receivers:
        message["To"] = email_receiver

        await aiosmtplib.send(message, conf.ad.smtp.email_sender, email_receiver,
                              conf.ad.smtp.host, conf.ad.smtp.port,
                              conf.ad.smtp.user, conf.ad.smtp.password,
                              start_tls=True)


async def run_dbscan(df: pd.DataFrame):
    data = df[["temperature", "power", "photoresistor"]]
    X = StandardScaler().fit(data)
    model: DBSCAN = DBSCAN(eps=0.5, min_samples=5).fit(X)

    # not sure if this works
    outliers_df = data[model.labels_ == -1]
    return outliers_df
