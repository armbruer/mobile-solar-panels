use esp_idf_hal::{
    gpio::{InputPin, OutputPin},
    i2c,
    i2c::I2c,
    i2c::Master,
    prelude::KiloHertz,
};
use esp_idf_sys::EspError;

use self::temperature::TemperatureSensor;

pub mod motor;
pub mod temperature;

pub struct I2CDevices<I2C: I2c, SDA: OutputPin + InputPin, SCL: OutputPin> {
    i2c: Master<I2C, SDA, SCL>,
    temperature_sensor: Option<TemperatureSensor<Master<I2C, SDA, SCL>>>,
    power_sensor: Option<ina219::INA219NonOwned<Master<I2C, SDA, SCL>>>,
}

impl<I2C: I2c, SDA: OutputPin + InputPin, SCL: OutputPin> I2CDevices<I2C, SDA, SCL> {
    pub fn new(
        i2c: I2C,
        i2c_pin_sda: SDA,
        i2c_pin_scl: SCL,
        enable_bmp180: bool,
        enable_ina219: bool,
    ) -> Result<I2CDevices<I2C, SDA, SCL>, EspError> {
        let config =
            <i2c::config::MasterConfig as Default>::default().baudrate(KiloHertz::from(400).into());
        let mut i2c_master = i2c::Master::new(
            i2c,
            i2c::MasterPins {
                sda: i2c_pin_sda,
                scl: i2c_pin_scl,
            },
            config,
        )?;

        let temperature_sensor = if enable_bmp180 {
            Some(TemperatureSensor::new(&mut i2c_master))
        } else {
            None
        };

        let power_sensor = if enable_ina219 {
            // todo!("Set address");
            Some(ina219::INA219NonOwned::new(&mut i2c_master, 0))
        } else {
            None
        };

        Ok(I2CDevices {
            i2c: i2c_master,
            temperature_sensor,
            power_sensor,
        })
    }

    pub fn get_temperature(&mut self) -> f32 {
        match &mut self.temperature_sensor {
            Some(temperature_sensor) => temperature_sensor.get_temperature(&mut self.i2c),
            None => todo!("Add exception handler"),
        }
    }

    pub fn get_pressure(&mut self) -> i32 {
        match &mut self.temperature_sensor {
            Some(temperature_sensor) => temperature_sensor.get_pressure(&mut self.i2c),
            None => todo!("Add exception handler"),
        }
    }

    pub fn get_power(&mut self) -> i16 {
        match &mut self.power_sensor {
            Some(power_sensor) => power_sensor.power(&mut self.i2c).expect("TODO"),
            None => todo!("Add exception handler"),
        }
    }

    pub fn get_voltage(&mut self) -> u16 {
        match &mut self.power_sensor {
            Some(power_sensor) => power_sensor.voltage(&mut self.i2c).expect("TODO"),
            None => todo!("Add exception handler"),
        }
    }

    pub fn get_current(&mut self) -> i16 {
        match &mut self.power_sensor {
            Some(power_sensor) => power_sensor.current(&mut self.i2c).expect("TODO"),
            None => todo!("Add exception handler"),
        }
    }

    pub fn get_calibrate(&mut self, value: u16) {
        match &mut self.power_sensor {
            Some(power_sensor) => power_sensor.calibrate(&mut self.i2c, value).expect("TODO"),
            None => todo!("Add exception handler"),
        }
    }

    pub fn get_shunt_voltage(&mut self) -> i16 {
        match &mut self.power_sensor {
            Some(power_sensor) => power_sensor.shunt_voltage(&mut self.i2c).expect("TODO"),
            None => todo!("Add exception handler"),
        }
    }
}
