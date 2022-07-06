import asyncio
import datetime
import logging
import threading

import aiocoap
import aiocoap.numbers.codes
import aiocoap.resource as resource
import suncalc

import model
from model import CommandState, DataPoint, Command, CommandTypes


class CommandResource(resource.Resource):
    command_state_lock: threading.Lock
    command_state: CommandState

    def __init__(self, command_state, command_state_lock):
        super().__init__()
        self.command_state = command_state
        self.command_state_lock = command_state_lock

        self.handle = None

    def get_link_description(self):
        # Publish additional data in .well-known/core
        return dict(**super().get_link_description(), title="Command pull resource.")

    async def render_get(self, request):
        self.command_state_lock.acquire()
        command_state = self.command_state
        self.command_state_lock.release()

        command = Command(CommandTypes.Nop, 0.0, 0.0)
        command.command = command_state.command
        if command_state.command == CommandTypes.Location:
            local_time = datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc).astimezone(
                command_state.local_timezone)
            sun_loc = suncalc.get_position(local_time, lng=command_state.longitude,
                                           lat=command_state.latitude)
            command.azimuth = sun_loc["azimuth"]
            command.altitude = sun_loc["altitude"]

        return aiocoap.Message(payload=command.serialize())


class SensorData(resource.Resource):
    received_data_points: asyncio.Queue

    def __init__(self, received_data_points):
        super().__init__()
        self.received_data_points = received_data_points

    def get_link_description(self):
        # Publish additional data in .well-known/core
        return dict(**super().get_link_description(), title="Sensor data upload resource.")

    async def render_get(self, request):
        return aiocoap.Message(payload=b"some response payload")

    async def render_post(self, request):
        logging.debug(f"POST received payload: {request.payload}")

        edge_current_time = datetime.datetime.utcnow()

        payload: bytes = request.payload
        length_size = 4
        if len(payload) < length_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST, payload=b"Minimum packet size is 4")

        length = int.from_bytes(payload[0:4], byteorder='little', signed=False)

        client_current_time_size = 8

        expected_packet_size = length_size + client_current_time_size + all_fields_size * length
        if len(payload) != expected_packet_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST,
                                   payload=b"Expected packet size: " + str(expected_packet_size).encode())

        client_current_time = int.from_bytes(payload[4:12], byteorder='little', signed=False)
        client_current_time = datetime.datetime.utcfromtimestamp(client_current_time)

        data = []

        index = length_size + client_current_time_size
        while index < len(payload):
            dp = DataPoint.deserialize(payload[index:index + model.DataPoint.get_serialized_size()])
            index += model.DataPoint.get_serialized_size()
            time_passed = client_current_time - dp.timestamp
            dp.timestamp = edge_current_time - time_passed

        assert index == len(payload)

        logging.debug("Sending datapoints to message queue...")
        await self.received_data_points.put(data)

        return aiocoap.Message(code=aiocoap.numbers.codes.Code.CHANGED, payload=b"ok")


async def run_coap(received_data_points: asyncio.Queue, command_state: CommandState,
                   command_state_lock: threading.Lock):
    # Resource tree creation
    root = resource.Site()
    root.add_resource(['.well-known', 'core'],
                      resource.WKCResource(root.get_resources_as_linkheader))
    root.add_resource(['command'], CommandResource(command_state, command_state_lock))
    root.add_resource(['sensor', 'data'], SensorData(received_data_points))

    logging.info("Creating CoAP server context")
    await aiocoap.Context.create_server_context(root)

    await asyncio.get_running_loop().create_future()
