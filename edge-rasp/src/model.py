import datetime
import enum
import struct


class CommandTypes(enum.Enum):
    Nop = 0
    Location = 1
    LightTracking = 2
    Stop = 3


class CommandState:
    command: CommandTypes
    latitude: float
    longitude: float
    local_timezone: datetime.timezone

    def __init__(self, command, latitude, longitude, local_timezone):
        self.command = command
        self.latitude = latitude
        self.longitude = longitude
        self.local_timezone = local_timezone

    def set_location_command_data(self, command, local_timezone, latitude, longitude):
        self.command = command
        self.local_timezone = local_timezone
        self.latitude = latitude
        self.longitude = longitude


class Command:
    command: CommandTypes
    azimuth: float
    altitude: float

    def __init__(self, command, azimuth, altitude):
        self.command = command
        self.azimuth = azimuth
        self.altitude = altitude

    def serialize(self) -> bytes:
        if self.command == CommandTypes.Location:
            return self.command.value.to_bytes(byteorder='little', signed=False, length=1) \
                   + struct.pack('<f', self.azimuth) + struct.pack('<f', self.altitude)
        else:
            return self.command.value.to_bytes(byteorder='little', signed=False, length=1)


class DataPoint:
    timestamp: datetime.datetime
    temperature: float
    photoresistor: int
    infrared: int
    voltage: int
    current: int
    power: int

    def __init__(self, timestamp, temperature, photoresistor, infrared, voltage, current, power):
        self.timestamp = timestamp
        self.temperature = temperature
        self.photoresistor = photoresistor
        self.infrared = infrared
        self.voltage = voltage
        self.current = current
        self.power = power

    def serialize(self) -> bytes:
        return int(self.timestamp.timestamp()).to_bytes(8, 'little', signed=False) + \
               struct.pack('<f', self.temperature) + \
               self.photoresistor.to_bytes(4, 'little', signed=False) + \
               self.infrared.to_bytes(4, 'little', signed=False) + \
               self.voltage.to_bytes(4, 'little', signed=False) + \
               self.current.to_bytes(4, 'little', signed=False) + \
               self.power.to_bytes(4, 'little', signed=False)

    @staticmethod
    def get_serialized_size():
        # timestamp + 4 * 6 (temperature, photoresistor, ir sensor, voltage, current, power)
        return 8 + 4 * 6

    @staticmethod
    def deserialize(payload: bytes):
        index = 0
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

        return DataPoint(timestamp=timestamp, temperature=temperature, photoresistor=photoresistor,
                               infrared=infrared, voltage=voltage, current=current, power=power)

    @staticmethod
    def aggregate_datapoints(datapoints):
        avg = lambda x: sum(x) / len(x)
        ts = datapoints[0].timestamp
        avg_tmp = avg(map(lambda dp: dp.temperature, datapoints))
        avg_pr = avg(map(lambda dp: dp.photoresistor, datapoints))
        avg_ir = avg(map(lambda dp: dp.infrared, datapoints))
        avg_volt = avg(map(lambda dp: dp.voltage, datapoints))
        avg_curr = avg(map(lambda dp: dp.current, datapoints))
        avg_pow = avg(map(lambda dp: dp.power, datapoints))
        return DataPoint(ts, avg_tmp, avg_pr, avg_ir, avg_volt, avg_curr, avg_pow)
