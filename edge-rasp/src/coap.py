import asyncio
import datetime
import logging
from copy import deepcopy

import aiocoap
import aiocoap.numbers.codes
import aiocoap.resource as resource
import suncalc

from model import CommandState, DataPoint, Command, CommandTypes


class CommandResource(resource.Resource):
    command_state_lock: asyncio.Lock
    command_state: CommandState

    def __init__(self, command_state, command_state_lock):
        super().__init__()
        self.command_state = command_state
        self.command_state_lock = command_state_lock

    def get_link_description(self):
        # Publish additional data in .well-known/core
        return dict(**super().get_link_description(), title="Command pull resource.")

    async def render_get(self, request):
        logging.debug("COAP: Acquiring lock...")
        await self.command_state_lock.acquire()

        assert len(request.payload) == 12
        device_id = int.from_bytes(request.payload[0:4], byteorder='little', signed=False)
        target_angle_offset_hor = int.from_bytes(request.payload[4:8], byteorder='little', signed=True)
        target_angle_offset_ver = int.from_bytes(request.payload[8:12], byteorder='little', signed=True)

        if self.command_state.leader_device_id is None:
            self.command_state.leader_device_id = device_id

        if self.command_state.leader_device_id == device_id:
            self.command_state.target_angle_offset_hor = target_angle_offset_hor
            self.command_state.target_angle_offset_ver = target_angle_offset_ver

        command_state = deepcopy(self.command_state)
        self.command_state_lock.release()
        logging.debug("COAP: Lock released")

        command = Command(CommandTypes.Nop, 0, 0, 0.0, 0.0)

        if command_state.leader_device_id == device_id:
            command.command = command_state.command
        # Handle follower devices
        else:
            if command_state.command in [CommandTypes.Location, CommandTypes.LightTracking]:
                command.command = CommandTypes.Follower
            else:
                command.command = command_state.command

        if command.command == CommandTypes.Location:
            # suncalc uses local_time.timestamp() and .timestamp() does not respect timezone
            # Therefore we add timezone information for calculations and then remove it once again
            local_time = datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc).astimezone(
                command_state.local_timezone).replace(tzinfo=datetime.timezone.utc)

            logging.debug(f"COAP: render_get(): Time: {local_time}, Longitude: {command_state.longitude}, "
                          f"Latitude: {command_state.latitude}")
            sun_loc = suncalc.get_position(local_time, lng=command_state.longitude,
                                           lat=command_state.latitude)
            command.azimuth = sun_loc["azimuth"]
            command.altitude = sun_loc["altitude"]
        elif command.command == CommandTypes.Follower:
            command.target_angle_offset_hor = command_state.target_angle_offset_hor
            command.target_angle_offset_ver = command_state.target_angle_offset_ver

        return aiocoap.Message(payload=command.serialize())


class SensorData(resource.Resource):
    received_data_points_db: asyncio.Queue
    received_data_points_mqtt: asyncio.Queue

    def __init__(self, received_data_points_db, received_data_points_mqtt):
        super().__init__()
        self.received_data_points_db = received_data_points_db
        self.received_data_points_mqtt = received_data_points_mqtt

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

        expected_packet_size = length_size + client_current_time_size + DataPoint.get_serialized_size() * length
        if len(payload) != expected_packet_size:
            return aiocoap.Message(code=aiocoap.numbers.codes.Code.BAD_REQUEST,
                                   payload=b"Expected packet size: " + str(expected_packet_size).encode())

        datapoints = self.parse_payload(client_current_time_size, edge_current_time, length_size, payload)

        logging.debug("Sending datapoints to queues...")
        await self.received_data_points_mqtt.put(datapoints)
        await self.received_data_points_db.put(datapoints)
        logging.debug("Sent datapoints to queues...")

        return aiocoap.Message(code=aiocoap.numbers.codes.Code.CHANGED, payload=b"ok")

    def parse_payload(self, client_current_time_size, edge_current_time, length_size, payload):
        client_current_time = int.from_bytes(payload[4:12], byteorder='little', signed=False)
        client_current_time = datetime.datetime.utcfromtimestamp(client_current_time)

        datapoints = []
        index = length_size + client_current_time_size

        while index < len(payload):
            dp = DataPoint.deserialize(payload[index:index + DataPoint.get_serialized_size()])
            index += DataPoint.get_serialized_size()
            time_passed = client_current_time - dp.timestamp
            dp.timestamp = edge_current_time - time_passed

            datapoints.append(dp)

        assert index == len(payload)
        return datapoints


async def run_coap(received_data_points_db: asyncio.Queue, received_data_points_mqtt: asyncio.Queue,
                   command_state: CommandState, command_state_lock: asyncio.Lock):
    # Resource tree creation
    root = resource.Site()
    root.add_resource(['.well-known', 'core'],
                      resource.WKCResource(root.get_resources_as_linkheader))
    root.add_resource(['command'], CommandResource(command_state, command_state_lock))
    root.add_resource(['sensor', 'data'], SensorData(received_data_points_db, received_data_points_mqtt))

    logging.info("Creating CoAP server context")
    await aiocoap.Context.create_server_context(root)

    await asyncio.get_running_loop().create_future()
