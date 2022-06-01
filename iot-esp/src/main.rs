mod sensors;

use esp_idf_hal::prelude::Peripherals;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::EspError;

use sensors::motor::StepperMotor;
use sensors::temperature::TemperatureSensor;

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    let lowSpeed = 10000; // max: 16000
    let highSpeed = 1000;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    let mut temp_sensor = TemperatureSensor::new(peripherals.i2c0, pins.gpio21, pins.gpio22)?;
    let mut step_motor1 = StepperMotor::new(
        pins.gpio16.into_output()?,
        pins.gpio17.into_output()?,
        pins.gpio18.into_output()?,
        pins.gpio19.into_output()?,
    )?;

    let mut step_motor2 = StepperMotor::new(
        pins.gpio12.into_output()?,
        pins.gpio14.into_output()?,
        pins.gpio27.into_output()?,
        pins.gpio26.into_output()?,
    )?;

    loop {
        log::info!("rotateRight");
        for _ in 0..5000 {
            step_motor1.rotateRight(highSpeed);
            step_motor2.rotateLeft(highSpeed);
        }

        step_motor1.stopMotor();
        step_motor2.stopMotor();
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
