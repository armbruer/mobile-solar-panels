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

    def set_location_command_data(self, local_timezone, latitude, longitude):
        self.local_timezone = local_timezone
        self.latitude = latitude
        self.longitude = longitude


class Command:
    command: CommandTypes
    azimuth: float
    altitude: float

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

    def __str__(self):
        return self.timestamp.isoformat() + " " + str(self.temperature) + " " + str(self.photoresistor) + " " \
               + str(self.infrared) + " " + str(self.voltage) + " " + str(self.current) + " " + str(self.power)

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
