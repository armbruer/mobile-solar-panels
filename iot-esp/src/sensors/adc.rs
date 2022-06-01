use embedded_hal::adc::{Channel, OneShot};
use esp_idf_hal::{
    adc,
    adc::{Adc, PoweredAdc},
};
use esp_idf_sys::EspError;

pub struct ADCMeasurement<ADC: Adc> {
    powered_adc: PoweredAdc<ADC>,
}

impl<ADC: Adc + adc::Analog<ADC>> ADCMeasurement<ADC> {
    pub fn new(adc: ADC) -> Result<ADCMeasurement<ADC>, EspError> {
        let powered_adc = adc::PoweredAdc::new(adc, adc::config::Config::new().calibration(true))?;

        Ok(ADCMeasurement { powered_adc })
    }

    pub fn get_voltage<Pin: Channel<ADC, ID = u8>>(&mut self, pin: &mut Pin) -> u16 {
        self.powered_adc.read(pin).unwrap()
    }
}
