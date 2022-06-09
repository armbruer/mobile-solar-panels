import datetime
import logging

import asyncio

import aiocoap.resource as resource
import aiocoap
import aiocoap.numbers.codes

import struct

logging.basicConfig(level=logging.INFO)
logging.getLogger("coap-server").setLevel(logging.DEBUG)

# TODO: Better use a channel
received_data_points = []


class TimeResource(resource.ObservableResource):
    """Example resource that can be observed. The `notify` method keeps
    scheduling itself, and calles `update_state` to trigger sending
    notifications."""

    def __init__(self):
        super().__init__()

        self.handle = None

    def notify(self):
        self.updated_state()
        self.reschedule()

    def reschedule(self):
        self.handle = asyncio.get_event_loop().call_later(5, self.notify)

    def update_observation_count(self, count):
        if count and self.handle is None:
            print("Starting the clock")
            self.reschedule()
        if count == 0 and self.handle:
            print("Stopping the clock")
            self.handle.cancel()
            self.handle = None

    async def render_get(self, request):
        payload = datetime.datetime.now(). \
            strftime("%Y-%m-%d %H:%M").encode('ascii')
        return aiocoap.Message(payload=payload)


class DataPoint:
    temperature: float
    photoresistor: int
    infrared: int

    def __init__(self, temperature, photoresistor, infrared):
        self.temperature = temperature
        self.photoresistor = photoresistor
        self.infrared = infrared


class SensorData(resource.Resource):
    def __init__(self):
        super().__init__()

    def get_link_description(self):
        # Publish additional data in .well-known/core
        return dict(**super().get_link_description(), title="Sensor data upload resource.")

    async def render_get(self, request):
        return aiocoap.Message(payload=b"some response payload")

    async def render_post(self, request):
        print(f"POST received payload: {request.payload}")

        payload: bytes = request.payload
        length_size = 4
        if len(payload) < length_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST, payload=b"Minimum packet size is 4")

        length = int.from_bytes(payload[0:4], byteorder='little', signed=False)

        all_fields_size = 4 * 3
        expected_packet_size = length_size + all_fields_size * length
        if len(payload) != expected_packet_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST,
                                   payload=b"Expected packet size: " + str(expected_packet_size).encode())

        data = []

        index = length_size
        while index < len(payload):
            temperature = struct.unpack('<f', payload[index:index+4])
            photoresistor = int.from_bytes(payload[index+4:index+8], byteorder='little', signed=True)
            infrared = int.from_bytes(payload[index+8:index+12], byteorder='little', signed=True)
            index += all_fields_size

            data_point = DataPoint(temperature=temperature, photoresistor=photoresistor, infrared=infrared)
            data.append(data_point)

        global received_data_points
        received_data_points += data

        return aiocoap.Message(code=aiocoap.numbers.codes.Code.CHANGED, payload=b"")


async def worker():
    pass


async def main():
    # Resource tree creation
    root = resource.Site()

    root.add_resource(['.well-known', 'core'],
                      resource.WKCResource(root.get_resources_as_linkheader))
    root.add_resource(['time'], TimeResource())
    root.add_resource(['sensor', 'data'], SensorData())

    await aiocoap.Context.create_server_context(root)

    # Run forever
    await asyncio.get_running_loop().create_future()

    await asyncio.gather(worker(), asyncio.get_running_loop().create_future())


if __name__ == "__main__":
    asyncio.run(main())
