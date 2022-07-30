from typing import Set

import aiosmtplib
import pandas as pd
import datetime

from email.message import EmailMessage
from model import Config


async def send_mail(conf: Config, outliers_df: pd.DataFrame):
    device_ids = outliers_df['device_id'].unique()
    device_ids.sort()
    devices = ', '.join(map(str, device_ids))

    outliers_dfs = [y for _, y in outliers_df.groupby('device_id', as_index=False)]
    anomalies = []
    for df in outliers_dfs:
        device_id = str(df.head(1)['device_id'].values[0])
        anomaly_dates = list(map(lambda x: x.isoformat(), df['time'].sort_values()))

        anomaly_datetimes = ', '.join(anomaly_dates)
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

        await aiosmtplib.send(message=message, sender=conf.anomaly_detection.smtp.email_sender,
                              recipients=email_receiver, hostname=conf.anomaly_detection.smtp.host,
                              port=conf.anomaly_detection.smtp.port, username=conf.anomaly_detection.smtp.user,
                              password=conf.anomaly_detection.smtp.password, use_tls=True)
