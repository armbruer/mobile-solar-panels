mod networking;
mod sensors;

use std::sync::Arc;
use std::time::Duration;

use adc_interpolator::AdcInterpolator;
use coap_lite::RequestType;
use esp_idf_hal::adc;
use esp_idf_hal::prelude::Peripherals;

use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::sysloop::EspSysLoopStack;
use esp_idf_sys::EspError;
use esp_idf_sys::{self as _, sleep}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use sensors::motor::StepperMotor;
use sensors::temperature::TemperatureSensor;
use networking::coap::Connection;

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    //esp_idf_svc::log::EspLogger.set_target_level("rust-logging", esp_idf_svc::log::Level::Debug);

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
    /*

    let pin_photoresistor = pins.gpio34.into_analog_atten_11db()?;
    let pin_ir_sensor = pins.gpio35.into_analog_atten_11db()?;
    let mut interpolator_photoresistor =
        AdcInterpolator::new(pin_photoresistor, config_photoresistor);
    let mut interpolator_ir_sensor = AdcInterpolator::new(pin_ir_sensor, config_ir_sensor);

    let mut powered_adc = adc::PoweredAdc::new(
        peripherals.adc1,
        adc::config::Config::new().calibration(true),
    )?;

    */

    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    // TODO hostname
    let _wifi = networking::wifi::wifi(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
    )?;

    /* 
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

    */
    let mut conn = Connection::new();

    loop {
        send_sensor_data(&mut conn, "10.42.0.1:5683", &vec![1.0, 2.0, 3.0], &vec![4, 5, 6], &vec![7, 8, 9]);
        log::info!("Sent a message");
        std::thread::sleep(Duration::from_secs(2));

    }
}


fn send_sensor_data(conn: &mut Connection, addr: &str, temperature: &[f32], photoresistor: &[i32], infrared: &[i32]) {
    let mut payload = vec![];
    
    // TODO prevent fragementation
    for ((t,p),i) in temperature.iter().zip(photoresistor.iter()).zip(infrared.iter()) {
        payload.extend_from_slice(&t.to_ne_bytes());
        payload.extend_from_slice(&p.to_ne_bytes());
        payload.extend_from_slice(&i.to_ne_bytes());
    }

    conn.send(RequestType::Post, addr, "/sensor/data", payload);
}