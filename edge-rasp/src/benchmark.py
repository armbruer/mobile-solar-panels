import datetime
import logging
import asyncio
from asyncio import futures

import aiocoap
from aiocoap import *

import model

logging.basicConfig(level=logging.INFO)


def generate_datapoint(device_id: int) -> model.DataPoint:
    return model.DataPoint(device_id, datetime.datetime.utcnow(), 123.123, 321, 456, 654, 789, 987)


def send_request(protocol: aiocoap.protocol.Context, target: str, device_id: int) -> futures.Future:
    data = [generate_datapoint(device_id)]

    payload = b""
    payload += len(data).to_bytes(4, byteorder='little', signed=False)
    payload += int(datetime.datetime.utcnow().timestamp()).to_bytes(8, byteorder='little', signed=False)
    for data_point in data:
        payload += data_point.serialize()

    request = Message(code=aiocoap.Code.POST, payload=payload, uri=target)

    return protocol.request(request).response


async def main():
    target = "coap://10.42.0.89/sensor/data"

    protocol = await Context.create_client_context()

    while True:
        start = datetime.datetime.utcnow()
        responses = [send_request(protocol, target, device_id) for device_id in range(1000, 2000)]
        await asyncio.gather(*responses)
        end = datetime.datetime.utcnow()
        print((end - start).total_seconds())


if __name__ == "__main__":
    asyncio.run(main())


"""
Output with 1000 concurrent requests:
5.931635
6.48877
5.314207
5.07413
5.448808
5.762699
5.031474
5.002175
5.085548
5.040889
4.947042
5.272289
5.976361
6.499246
5.300084
5.203298
5.246041
5.120356
4.976372
5.009133
4.999139
6.008205
4.86652
5.435855
5.424017
6.662154
5.387039
4.978077
5.095268
4.949785

-> Average: 5.38s
"""
