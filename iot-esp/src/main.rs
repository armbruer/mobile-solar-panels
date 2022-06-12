mod networking;
mod sensors;

use std::sync::Arc;
use std::time::Duration;

use adc_interpolator::AdcInterpolator;
use coap_lite::RequestType;
use esp_idf_hal::adc;
use esp_idf_hal::gpio::{Gpio34, Gpio35};
use esp_idf_hal::prelude::Peripherals;

use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::sysloop::EspSysLoopStack;
use esp_idf_sys::EspError;
use esp_idf_sys::{self as _, sleep}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

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

    let mut stepper_motor1 = StepperMotor::new(
        pins.gpio16.into_output()?,
        pins.gpio17.into_output()?,
        pins.gpio18.into_output()?,
        pins.gpio19.into_output()?,
        180.0, //TODO calibrate
        0.72,  // 1.8   //TODO to be determined
    );

    let mut stepper_motor2 = StepperMotor::new(
        pins.gpio12.into_output()?,
        pins.gpio14.into_output()?,
        pins.gpio27.into_output()?,
        pins.gpio26.into_output()?,
        180.0, //TODO calibrate
        0.72,  //1.8   //TODO to be determined
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
    let mut interpolator_photoresistor: AdcInterpolator<Gpio34<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_photoresistor, config_photoresistor);
    let mut interpolator_ir_sensor_1: AdcInterpolator<Gpio35<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_ir_sensor, config_ir_sensor);

    let mut powered_adc = adc::PoweredAdc::new(
        peripherals.adc1,
        adc::config::Config::new().calibration(true),
    )?;

    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    // Main motor algorithm
    let motor_control = false;
    if motor_control {
        init_motors(stepper_motor1, stepper_motor2, interpolator_ir_sensor, powered_adc);
        search_vague(stepper_motor1, stepper_motor2, interpolator_photoresistor, powered_adc);
        search_exact(stepper_motor1, stepper_motor2, interpolator_photoresistor, powered_adc);
        follow_sun(stepper_motor1, stepper_motor2, interpolator_photoresistor, powered_adc);
    }

    // Demo: Hardware measurements on serial port and motors turning
    let demo_hardware_measurements = false;
    if demo_hardware_measurements {
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
                i2c_sensors.get_temperature(),
                i2c_sensors.get_pressure()
            );

            std::thread::sleep(std::time::Duration::from_secs(2));
        });

        thread_stepper_motor1.join().unwrap();
        thread_stepper_motor2.join().unwrap();
        thread_measure.join().unwrap();
    }

    let demo_coap = false;
    if demo_coap {
        // TODO hostname
        let _wifi = networking::wifi::wifi(
            netif_stack.clone(),
            sys_loop_stack.clone(),
            default_nvs.clone(),
        )?;

        let mut conn = Connection::new();

        loop {
            send_sensor_data(
                &mut conn,
                "10.42.0.1:5683",
                &vec![1.0, 2.0, 3.0],
                &vec![4, 5, 6],
                &vec![7, 8, 9],
            );
            log::info!("Sent a message");
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    Ok(())
}

fn init_motors(
    stepper_motor1: StepperMotor,
    stepper_motor2: StepperMotor,
    interpolator_ir_sensor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
) {
    let ir_sensor_data_close1 = 0.0; //TODO calibrate
    let ir_sensor_data_close2 = 0.0; //TODO calibrate

    // init stepper_motor1 angle
    while interpolator_ir_sensor.read(&mut powered_adc).unwrap() < ir_sensor_data_close1 {
        stepper_motor1.rotateLeft(sensors::motor::Speed::LowSpeed);
    }
    stepper_motor1.init(false);
    stepper_motor1.stopMotor();

    // init stepper_motor2 angle
    while interpolator_ir_sensor.read(&mut powered_adc).unwrap() < ir_sensor_data_close2
    //maybe use a second ir sensor
    {
        stepper_motor2.rotateLeft(sensors::motor::Speed::LowSpeed);
    }
    stepper_motor2.init(false);
    stepper_motor2.stopMotor();
}

fn search_vague(
    stepper_motor1: StepperMotor,
    stepper_motor2: StepperMotor,
    interpolator_photoresistor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
) {
    //search for the sun by moving the motors in an ⧖ shape
    let angle1 = stepper_motor1.current_angle;
    let angle2 = stepper_motor2.current_angle;
    let photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
    let best_position = vec![photoresistor, angle1, angle2];

    //1. line of the ⧖ shape
    while stepper_motor2.rotatableRight() {
        angle2 = stepper_motor2.rotateRight(sensors::motor::Speed::LowSpeed);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //2. line of the ⧖ shape
    let half_max_angle = stepper_motor1.max_angle / 2;
    while stepper_motor1.rotatableAngle(half_max_angle) {
        angle1 = stepper_motor1.rotateAngle(sensors::motor::Speed::LowSpeed, half_max_angle);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //3. line of the ⧖ shape
    while stepper_motor2.rotatableLeft() {
        angle2 = stepper_motor2.rotateLeft(sensors::motor::Speed::LowSpeed);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //4. line of the ⧖ shape
    while stepper_motor1.rotatableRight() {
        angle1 = stepper_motor1.rotateRight(sensors::motor::Speed::LowSpeed);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //move to best position
    stepper_motor1.rotateAngleFull(sensors::motor::Speed::HighSpeed, best_position[1]);
    stepper_motor2.rotateAngleFull(sensors::motor::Speed::HighSpeed, best_position[2]);
}

fn search_exact(
    stepper_motor1: StepperMotor,
    stepper_motor2: StepperMotor,
    interpolator_photoresistor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
) {
    //search for the sun within a grid around the current position
    let gridsize = 7; //TODO calibrate
    let angle1 = stepper_motor1.current_angle;
    let angle2 = stepper_motor2.current_angle;
    let photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
    let init_best_position = vec![photoresistor, angle1, angle2];
    let new_best_position = init_best_position;

    //repeat until best position is reached
    loop
    {
        //move to the leftest position of both motors
        let half_gridsize = (gridsize+1) / 2;
        for _ in 1..half_gridsize {
            angle1 = stepper_motor1.rotateLeft(sensors::motor::Speed::HighSpeed);
            angle2 = stepper_motor2.rotateLeft(sensors::motor::Speed::HighSpeed);
        }

        //go through each position in the grid in wavy lines and check if one is better than before
        for m1 in 1..gridsize {
            for m2 in 1..gridsize {
                if m2 % 2 == 0
                {
                    angle2 = stepper_motor2.rotateLeft(sensors::motor::Speed::LowSpeed);
                }
                else
                {
                    angle2 = stepper_motor2.rotateRight(sensors::motor::Speed::LowSpeed);
                }
                photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
                if new_best_position[0] < photoresistor {
                    new_best_position = vec![photoresistor, angle1, angle2];
                }
            }
            angle1 = stepper_motor1.rotateRight(sensors::motor::Speed::LowSpeed);
        }

        //move to best position
        stepper_motor1.rotateAngleFull(sensors::motor::Speed::HighSpeed, new_best_position[1]);
        stepper_motor2.rotateAngleFull(sensors::motor::Speed::HighSpeed, new_best_position[2]);

        //stop if best position is reached
        if new_best_position == init_best_position
        {
            break;
        }
    }
}

fn follow_sun(stepper_motor1: StepperMotor,
    stepper_motor2: StepperMotor,
    interpolator_photoresistor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
) {

}

fn send_sensor_data(
    conn: &mut Connection,
    addr: &str,
    temperature: &[f32],
    photoresistor: &[i32],
    infrared: &[i32],
) {
    let mut payload = vec![];

    debug_assert_eq!(temperature.len(), photoresistor.len());
    debug_assert_eq!(photoresistor.len(), infrared.len());

    payload.extend_from_slice(&(temperature.len() as u32).to_ne_bytes());

    // TODO prevent fragementation
    for ((t, p), i) in temperature
        .iter()
        .zip(photoresistor.iter())
        .zip(infrared.iter())
    {
        payload.extend_from_slice(&t.to_le_bytes());
        payload.extend_from_slice(&p.to_le_bytes());
        payload.extend_from_slice(&i.to_le_bytes());
    }

    conn.send(RequestType::Post, addr, "/sensor/data", payload);
}
