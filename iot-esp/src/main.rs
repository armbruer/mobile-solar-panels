mod sensors;

use esp_idf_hal::prelude::Peripherals;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::EspError;

use sensors::temperature::TemperatureSensor;

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    let mut temp_sensor = TemperatureSensor::new(peripherals.i2c0, pins.gpio21, pins.gpio22)?;

    loop {
        log::info!(
            "Temperature sensor reading: {}°C",
            temp_sensor.get_temperature().unwrap()
        );

        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
