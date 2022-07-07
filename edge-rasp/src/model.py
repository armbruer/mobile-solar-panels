import datetime
import enum
import struct
from copy import deepcopy

from dataclasses import dataclass
from typing import Optional


class CommandTypes(enum.Enum):
    Nop = 0
    Location = 1
    LightTracking = 2
    Follower = 3
    Stop = 4


@dataclass
class CommandState:
    command: CommandTypes
    target_angle_offset_hor: int
    target_angle_offset_ver: int
    latitude: float
    longitude: float
    local_timezone: datetime.timezone

    leader_device_id: Optional[int]

    @staticmethod
    def default():
        return CommandState(CommandTypes.Nop, 0, 0, 0.0, 0.0, datetime.timezone.utc, None)

    def __copy__(self):
        return type(CommandState)(self.command, self.latitude, self.longitude, self.local_timezone)

    def __deepcopy__(self, memo):  # memo is a dict of id's to copies
        id_self = id(self)  # memoization avoids unnecessary recursion
        _copy = memo.get(id_self)
        if _copy is None:
            _copy = type(CommandState)(
                deepcopy(self.command, memo),
                deepcopy(self.latitude, memo),
                deepcopy(self.longitude, memo),
                deepcopy(self.local_timezone, memo))
            memo[id_self] = _copy
        return _copy

    def set_location_command_data(self, local_timezone, latitude, longitude):
        self.local_timezone = local_timezone
        self.latitude = latitude
        self.longitude = longitude


@dataclass
class Command:
    command: CommandTypes
    target_angle_offset_hor: int
    target_angle_offset_ver: int
    azimuth: float
    altitude: float

    def serialize(self) -> bytes:
        if self.command == CommandTypes.Location:
            return self.command.value.to_bytes(byteorder='little', signed=False, length=1) \
                   + struct.pack('<f', self.azimuth) + struct.pack('<f', self.altitude)
        elif self.command == CommandTypes.Follower:
            return self.command.value.to_bytes(byteorder='little', signed=False, length=1) \
                   + self.target_angle_offset_hor.to_bytes(byteorder='little', signed=True, length=4) \
                   + self.target_angle_offset_ver.to_bytes(byteorder='little', signed=True, length=4)
        else:
            return self.command.value.to_bytes(byteorder='little', signed=False, length=1)


@dataclass
class DataPoint:
    # unique identifier of ESP device
    device_id: int
    timestamp: datetime.datetime
    temperature: float
    photoresistor: int
    infrared: int
    voltage: int
    current: int
    power: int

    def serialize(self) -> bytes:
        return self.device_id.to_bytes(4, 'little', signed=False) + \
               int(self.timestamp.timestamp()).to_bytes(8, 'little', signed=False) + \
               struct.pack('<f', self.temperature) + \
               self.photoresistor.to_bytes(4, 'little', signed=False) + \
               self.infrared.to_bytes(4, 'little', signed=False) + \
               self.voltage.to_bytes(4, 'little', signed=False) + \
               self.current.to_bytes(4, 'little', signed=False) + \
               self.power.to_bytes(4, 'little', signed=False)

    @staticmethod
    def get_serialized_size():
        # device_id + timestamp + 4 * 6 (temperature, photoresistor, ir sensor, voltage, current, power)
        return 4 + 8 + 4 * 6

    @staticmethod
    def deserialize(payload: bytes):
        index = 0
        device_id = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
        index += 4
        timestamp = int.from_bytes(payload[index:index + 8], byteorder='little', signed=False)
        timestamp = datetime.datetime.utcfromtimestamp(timestamp)
        index += 8
        temperature = struct.unpack('<f', payload[index:index + 4])[0]
        index += 4
        photoresistor = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
        index += 4
        infrared = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
        index += 4
        voltage = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
        index += 4
        current = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
        index += 4
        power = int.from_bytes(payload[index:index + 4], byteorder='little', signed=False)
        index += 4

        assert index == len(payload)

        return DataPoint(device_id=device_id, timestamp=timestamp, temperature=temperature, photoresistor=photoresistor,
                         infrared=infrared, voltage=voltage, current=current, power=power)

    @staticmethod
    def aggregate_datapoints(datapoints):
        def avg(x): return sum(x) / len(x)

        device_id = datapoints[0].device_id
        timestamp = datapoints[0].timestamp

        avg_temperature = avg(list(map(lambda dp: dp.temperature, datapoints)))
        avg_photoresistor = int(avg(list(map(lambda dp: dp.photoresistor, datapoints))))
        avg_infrared = int(avg(list(map(lambda dp: dp.infrared, datapoints))))
        avg_voltage = int(avg(list(map(lambda dp: dp.voltage, datapoints))))
        avg_current = int(avg(list(map(lambda dp: dp.current, datapoints))))
        avg_power = int(avg(list(map(lambda dp: dp.power, datapoints))))

        return DataPoint(device_id=device_id, timestamp=timestamp, temperature=avg_temperature,
                         photoresistor=avg_photoresistor, infrared=avg_infrared, voltage=avg_voltage,
                         current=avg_current, power=avg_power)
