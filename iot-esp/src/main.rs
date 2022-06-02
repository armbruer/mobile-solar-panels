mod sensors;

use adc_interpolator::AdcInterpolator;
use esp_idf_hal::adc;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::EspError;

use sensors::motor::StepperMotor;
use sensors::temperature::TemperatureSensor;

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let mut temp_sensor = TemperatureSensor::new(peripherals.i2c0, pins.gpio21, pins.gpio22)?;

    let mut stepper_motor1 = StepperMotor::new(
        pins.gpio16.into_output()?,
        pins.gpio17.into_output()?,
        pins.gpio18.into_output()?,
        pins.gpio19.into_output()?,
    );

    let mut stepper_motor2 = StepperMotor::new(
        pins.gpio12.into_output()?,
        pins.gpio14.into_output()?,
        pins.gpio27.into_output()?,
        pins.gpio26.into_output()?,
    );

    let config_photoresistor = adc_interpolator::Config {
        max_voltage: 3300, // 3300 mV maximum voltage
        precision: 12,     // 12-bit precision
        voltage_to_values: [
            (100, 100), // Above 3000 mV and below 200 mV is likely invalid
            (2000, 2000),
            (3500, 3500),
        ],
    };

    let config_ir_sensor = adc_interpolator::Config {
        max_voltage: 3300, // 3300 mV maximum voltage
        precision: 12,     // 12-bit precision
        voltage_to_values: [
            (100, 100), // Above 3000 mV and below 200 mV is likely invalid
            (2000, 2000),
            (3500, 3500),
        ],
    };

    let pin_photoresistor = pins.gpio34.into_analog_atten_11db()?;
    let pin_ir_sensor = pins.gpio35.into_analog_atten_11db()?;
    let mut interpolator_photoresistor =
        AdcInterpolator::new(pin_photoresistor, config_photoresistor);
    let mut interpolator_ir_sensor = AdcInterpolator::new(pin_ir_sensor, config_ir_sensor);

    let mut powered_adc = adc::PoweredAdc::new(
        peripherals.adc1,
        adc::config::Config::new().calibration(true),
    )?;

    let thread_stepper_motor1 = std::thread::spawn(move || {
        for _ in 0..20 {
            for _ in 0..200 {
                stepper_motor1.rotateRight(sensors::motor::Speed::HighSpeed);
            }
            stepper_motor1.stopMotor();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
    let thread_stepper_motor2 = std::thread::spawn(move || {
        for _ in 0..20 {
            for _ in 0..200 {
                stepper_motor2.rotateLeft(sensors::motor::Speed::HighSpeed);
            }
            stepper_motor2.stopMotor();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    let thread_measure = std::thread::spawn(move || loop {
        let photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        let ir_sensor = interpolator_ir_sensor.read(&mut powered_adc).unwrap();

        log::info!(
            "Photoresitor: {:#?}, IR Sensor: {:#?}",
            photoresistor,
            ir_sensor
        );

        log::info!(
            "Temperature: {} °C, Pressure: {} Pa",
            temp_sensor.get_temperature(),
            temp_sensor.get_pressure()
        );

        std::thread::sleep(std::time::Duration::from_secs(2));
    });

    thread_stepper_motor1.join().unwrap();
    thread_stepper_motor2.join().unwrap();
    thread_measure.join().unwrap();

    Ok(())
}
