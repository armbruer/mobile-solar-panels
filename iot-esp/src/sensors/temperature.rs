use embedded_drivers::bmp180::BMP180NonOwned;
use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

pub struct TemperatureSensor<I2C> {
    bmp180: BMP180NonOwned<I2C>,
}

impl<I2C: Write + WriteRead + Read> TemperatureSensor<I2C> {
    pub fn new(i2c: &mut I2C) -> TemperatureSensor<I2C> {
        let mut bmp180 = embedded_drivers::bmp180::BMP180NonOwned::new(i2c);
        bmp180.init(i2c);

        TemperatureSensor { bmp180 }
    }

    pub fn get_temperature(&mut self, i2c: &mut I2C) -> f32 {
        self.bmp180
            .get_temperature(i2c, &mut esp_idf_hal::delay::Ets)
    }

    pub fn get_pressure(&mut self, i2c: &mut I2C) -> i32 {
        self.bmp180.get_pressure(i2c, &mut esp_idf_hal::delay::Ets)
    }
}
