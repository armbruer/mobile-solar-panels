mod control;
mod networking;
mod sensors;

use std::sync::Arc;
use std::time::Duration;

use adc_interpolator::AdcInterpolator;
use coap_lite::RequestType;
use control::lighttracking::Platform;
use esp_idf_hal::adc;
use esp_idf_hal::gpio::{Gpio32, Gpio34, Gpio35};
use esp_idf_hal::prelude::Peripherals;

use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::sysloop::EspSysLoopStack;
use esp_idf_sys::EspError;
use esp_idf_sys::{self as _}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use networking::coap::Connection;
use sensors::motor::StepperMotor;

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    //esp_idf_svc::log::EspLogger.set_target_level("rust-logging", esp_idf_svc::log::Level::Debug);

    let mut i2c_sensors =
        sensors::I2CDevices::new(peripherals.i2c0, pins.gpio21, pins.gpio22, true, false)?;

    // 480 steps = 360°
    let stepper_motor_ver = StepperMotor::new(
        pins.gpio16.into_output()?,
        pins.gpio17.into_output()?,
        pins.gpio18.into_output()?,
        pins.gpio19.into_output()?,
        480 / 3, // TODO calibrate
        1,       // TODO to be determined
        0,
        true,
    );

    // 480 steps = 360°
    let stepper_motor_hor = StepperMotor::new(
        pins.gpio12.into_output()?,
        pins.gpio14.into_output()?,
        pins.gpio27.into_output()?,
        pins.gpio26.into_output()?,
        480, // TODO calibrate
        1,   // TODO to be determined
        0,
        true,
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

    let config_button_sensor = adc_interpolator::Config {
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
    let pin_button_sensor = pins.gpio32.into_analog_atten_11db()?;
    let mut interpolator_photoresistor: AdcInterpolator<Gpio34<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_photoresistor, config_photoresistor);
    let mut interpolator_ir_sensor_1: AdcInterpolator<Gpio35<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_ir_sensor, config_ir_sensor);
    let mut interpolator_button_sensor: AdcInterpolator<Gpio32<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_button_sensor, config_button_sensor);

    let mut powered_adc = adc::PoweredAdc::new(
        peripherals.adc1,
        adc::config::Config::new().calibration(true),
    )?;

    // Main motor algorithm
    let motor_control = true;
    if motor_control {
        let mut platform1 = Platform::new(
            stepper_motor_ver,
            stepper_motor_hor,
            interpolator_ir_sensor_1,
            interpolator_photoresistor,
            interpolator_button_sensor,
        );

        // For now the initial position at angle 0 is assumed
        // platform1.init_motors(&mut powered_adc);

        platform1.find_best_position(&mut powered_adc).unwrap();

        log::info!("Waiting for button press to terminate");
        while !platform1.reset_if_button_pressed(&mut powered_adc) {
            std::thread::sleep(Duration::from_millis(100));
        }

        return Ok(());

        let gridsize = 7; //TODO calibrate
        platform1
            .search_exact(
                &mut powered_adc,
                true,
                true,
                gridsize,
                gridsize,
                true,
                true,
                false,
            )
            .unwrap();
        platform1.follow_light(&mut powered_adc, gridsize);
    }
    // Demo: Hardware measurements on serial port and motors turning
    /*
    let demo_hardware_measurements = false;
    if demo_hardware_measurements {
        let thread_stepper_motor_ver = std::thread::spawn(move || {
            for _ in 0..20 {
                for _ in 0..200 {
                    stepper_motor_ver.rotate_right(sensors::motor::Speed::HighSpeed);
                }
                stepper_motor_ver.stop_motor();
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
        let thread_stepper_motor_hor = std::thread::spawn(move || {
            for _ in 0..20 {
                for _ in 0..200 {
                    stepper_motor_hor.rotate_left(sensors::motor::Speed::HighSpeed);
                }
                stepper_motor_hor.stop_motor();
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        let thread_measure = std::thread::spawn(move || loop {
            let photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
            let ir_sensor = interpolator_ir_sensor_1.read(&mut powered_adc).unwrap();

            log::info!(
                "Photoresitor: {:#?}, IR Sensor: {:#?}",
                photoresistor,
                ir_sensor
            );

            log::info!(
                "Temperature: {} °C, Pressure: {} Pa",
                i2c_sensors.get_temperature(),
                i2c_sensors.get_pressure()
            );

            std::thread::sleep(std::time::Duration::from_secs(2));
        });

        thread_stepper_motor_ver.join().unwrap();
        thread_stepper_motor_hor.join().unwrap();
        thread_measure.join().unwrap();
    }
    */

    let demo_coap = false;
    if demo_coap {
        // TODO hostname
        let _wifi = networking::wifi::wifi(
            Arc::new(EspNetifStack::new()?),
            Arc::new(EspSysLoopStack::new()?),
            Arc::new(EspDefaultNvs::new()?),
        );

        let mut coap_conn = Connection::new();

        loop {
            send_sensor_data(
                &mut coap_conn,
                "10.0.100.1:5683",
                &[
                    std::time::SystemTime::now(),
                    std::time::SystemTime::now(),
                    std::time::SystemTime::now(),
                ],
                &[1.0, 2.0, 3.0],
                &[4, 5, 6],
                &[7, 8, 9],
            );
            std::thread::sleep(Duration::from_secs(10));
        }
    }

    Ok(())
}

fn send_sensor_data(
    conn: &mut Connection,
    addr: &str,
    timestamp: &[std::time::SystemTime],
    temperature: &[f32],
    photoresistor: &[i32],
    infrared: &[i32],
) {
    // Assert all data parameters have same length
    debug_assert_eq!(timestamp.len(), temperature.len());
    debug_assert_eq!(temperature.len(), photoresistor.len());
    debug_assert_eq!(photoresistor.len(), infrared.len());

    // length_s + timestamp_s + datasets_length * (timestamp + temperature + photoresistor + IRsensor)
    let mut payload = vec![0; 4 + 8 + timestamp.len() * (8 + 4 * 3)];

    let mut index = 0;
    // 4 bytes: Amount of datasets in payload
    payload[index..index + 4].copy_from_slice(&(timestamp.len() as u32).to_le_bytes());
    index += 4;
    // 8 bytes: Placeholder for current SystemTime
    payload[index..index + 8].copy_from_slice(&0u64.to_le_bytes());
    index += 8;

    // TODO: prevent fragementation
    for (((time, temp), photo), infra) in timestamp
        .iter()
        .zip(temperature.iter())
        .zip(photoresistor.iter())
        .zip(infrared.iter())
    {
        let unix_time = time
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        payload[index..index + 8].copy_from_slice(&unix_time.to_le_bytes());
        index += 8;
        payload[index..index + 4].copy_from_slice(&temp.to_le_bytes());
        index += 4;
        payload[index..index + 4].copy_from_slice(&photo.to_le_bytes());
        index += 4;
        payload[index..index + 4].copy_from_slice(&infra.to_le_bytes());
        index += 4;
    }

    debug_assert_eq!(index, payload.len());

    // Set current SystemTime as reference for the other timestamps
    payload[4..4 + 8].copy_from_slice(
        &std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_le_bytes(),
    );

    match conn.request(RequestType::Post, addr, "/sensor/data", payload) {
        Ok(_) => (),
        Err(e) => log::error!("{:?}", e), // TODO: Store data that it doesn't get lost
    }
}
