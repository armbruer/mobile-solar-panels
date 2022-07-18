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

MIN_DATAPOINTS = 20
ANOMALY_DETECTION_INTERVAL = datetime.timedelta(minutes=10).total_seconds()
REPORT_INTERVAL = datetime.timedelta(minutes=180).total_seconds()


async def run_anomaly_detection(pool: asyncpg.Pool, conf: Config):
    if not (REPORT_INTERVAL % ANOMALY_DETECTION_INTERVAL == 0):
        logging.critical(f"REPORT_INTERVAL '{REPORT_INTERVAL}' must be a multiple of ANOMALY_DETECTION_INTERVAL"
                         f"'{ANOMALY_DETECTION_INTERVAL}'")

    await worker(pool, conf)


async def worker(pool: asyncpg.Pool, conf: Config):
    outliers_dfs = pd.DataFrame()
    rounds = REPORT_INTERVAL / ANOMALY_DETECTION_INTERVAL
    curr_round = 0

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
    devices = ', '.join(map(str, set(outliers_df.sort_values(by=['device_id'])['device_id'].tolist())))

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
