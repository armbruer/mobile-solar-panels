mod sensors;

use esp_idf_hal::prelude::Peripherals;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::EspError;

use sensors::temperature::TemperatureSensor;

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    let lowSpeed  = 10000; // max: 16000
    let highSpeed =  1000;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    let mut temp_sensor = TemperatureSensor::new(peripherals.i2c0, pins.gpio21, pins.gpio22)?;
    let mut step_motor = StepperMotor::new( pins.gpio17, pins.gpio18, pins.gpio19, pins.gpio20)?;

    loop {
        log::info!(
            "Temperature sensor reading: {}°C",
            temp_sensor.get_temperature().unwrap()
        );

        step_motor.stopMotor()
        step_motor.rotateRight(lowSpeed)
        step_motor.stopMotor()
        step_motor.rotateLeft(lowSpeed)
        step_motor.stopMotor()
        step_motor.rotateRight(highSpeed)
        step_motor.stopMotor()
        step_motor.rotateLeft(highSpeed)

        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
