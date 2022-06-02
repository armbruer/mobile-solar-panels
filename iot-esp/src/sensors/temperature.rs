use embedded_drivers::bmp180::BMP180;
use esp_idf_hal::{
    gpio::{InputPin, OutputPin},
    i2c,
    i2c::{I2c, Master},
    prelude::KiloHertz,
};
use esp_idf_sys::EspError;

pub struct TemperatureSensor<I2C: I2c, SDA: OutputPin + InputPin, SCL: OutputPin> {
    bmp180: BMP180<Master<I2C, SDA, SCL>>,
}

impl<I2C: I2c, SDA: OutputPin + InputPin, SCL: OutputPin> TemperatureSensor<I2C, SDA, SCL> {
    pub fn new(
        i2c: I2C,
        i2c_pin_sda: SDA,
        i2c_pin_scl: SCL,
    ) -> Result<TemperatureSensor<I2C, SDA, SCL>, EspError> {
        let config =
            <i2c::config::MasterConfig as Default>::default().baudrate(KiloHertz::from(400).into());
        let i2c_master = i2c::Master::new(
            i2c,
            i2c::MasterPins {
                sda: i2c_pin_sda,
                scl: i2c_pin_scl,
            },
            config,
        )?;

        let bmp180 = embedded_drivers::bmp180::BMP180::new(i2c_master);

        Ok(TemperatureSensor { bmp180 })
    }

    pub fn get_temperature(&mut self) -> f32 {
        self.bmp180.get_temperature(&mut esp_idf_hal::delay::Ets)
    }

    pub fn get_pressure(&mut self) -> i32 {
        self.bmp180.get_pressure(&mut esp_idf_hal::delay::Ets)
    }
}
