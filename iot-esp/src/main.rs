mod networking;
mod sensors;

use std::sync::Arc;
use std::time::Duration;
use std::cmp;

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

    let mut stepper_motor_ver = StepperMotor::new(
        pins.gpio16.into_output()?,
        pins.gpio17.into_output()?,
        pins.gpio18.into_output()?,
        pins.gpio19.into_output()?,
        180.0, //TODO calibrate
        0.72,  // 1.8   //TODO to be determined
    );

    let mut stepper_motor_hor = StepperMotor::new(
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
        init_motors(stepper_motor_ver, stepper_motor_hor, interpolator_ir_sensor, powered_adc);
        search_vague(stepper_motor_ver, stepper_motor_hor, interpolator_photoresistor, powered_adc);
        let gridsize = 7; //TODO calibrate
        let seach_result = search_exact(stepper_motor_ver, stepper_motor_hor, interpolator_photoresistor, powered_adc, true, true, gridsize, gridsize, true, true);
        follow_sun(stepper_motor_ver, stepper_motor_hor, interpolator_photoresistor, powered_adc, search_result.1, seach_result.2, gridsize);
    }

    // Demo: Hardware measurements on serial port and motors turning
    let demo_hardware_measurements = false;
    if demo_hardware_measurements {
        let thread_stepper_motor_ver = std::thread::spawn(move || {
            for _ in 0..20 {
                for _ in 0..200 {
                    stepper_motor_ver.rotateRight(sensors::motor::Speed::HighSpeed);
                }
                stepper_motor_ver.stopMotor();
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
        let thread_stepper_motor_hor = std::thread::spawn(move || {
            for _ in 0..20 {
                for _ in 0..200 {
                    stepper_motor_hor.rotateLeft(sensors::motor::Speed::HighSpeed);
                }
                stepper_motor_hor.stopMotor();
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

        thread_stepper_motor_ver.join().unwrap();
        thread_stepper_motor_hor.join().unwrap();
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
    stepper_motor_ver: StepperMotor,
    stepper_motor_hor: StepperMotor,
    interpolator_ir_sensor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
) {
    let ir_sensor_data_close1 = 0.0; //TODO calibrate
    let ir_sensor_data_close2 = 0.0; //TODO calibrate

    // init stepper_motor_ver angle
    while interpolator_ir_sensor.read(&mut powered_adc).unwrap() < ir_sensor_data_close1 {
        stepper_motor_ver.rotateLeft(sensors::motor::Speed::LowSpeed);
    }
    stepper_motor_ver.initAngle(false);
    stepper_motor_ver.stopMotor();

    // init stepper_motor_hor angle
    while interpolator_ir_sensor.read(&mut powered_adc).unwrap() < ir_sensor_data_close2
    //maybe use a second ir sensor
    {
        stepper_motor_hor.rotateLeft(sensors::motor::Speed::LowSpeed);
    }
    stepper_motor_hor.initAngle(false);
    stepper_motor_hor.stopMotor();
}

fn search_vague(
    stepper_motor_ver: StepperMotor,
    stepper_motor_hor: StepperMotor,
    interpolator_photoresistor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
) {
    //search for the sun by moving the motors in an ⧖ shape
    let angle1 = stepper_motor_ver.current_angle;
    let angle2 = stepper_motor_hor.current_angle;
    let photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
    let best_position = vec![photoresistor, angle1, angle2];

    //1. line of the ⧖ shape
    while stepper_motor_hor.rotatableRight() {
        angle2 = stepper_motor_hor.rotateRight(sensors::motor::Speed::LowSpeed);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //2. line of the ⧖ shape
    let half_max_angle = stepper_motor_ver.max_angle / 2;
    while stepper_motor_ver.rotatableAngle(half_max_angle) {
        angle1 = stepper_motor_ver.rotateAngle(sensors::motor::Speed::LowSpeed, half_max_angle);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //3. line of the ⧖ shape
    while stepper_motor_hor.rotatableLeft() {
        angle2 = stepper_motor_hor.rotateLeft(sensors::motor::Speed::LowSpeed);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //4. line of the ⧖ shape
    while stepper_motor_ver.rotatableRight() {
        angle1 = stepper_motor_ver.rotateRight(sensors::motor::Speed::LowSpeed);
        photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
        if best_position[0] < photoresistor {
            best_position = vec![photoresistor, angle1, angle2];
        }
    }
    //move to best position
    stepper_motor_ver.rotateAngleFull(sensors::motor::Speed::HighSpeed, best_position[1]);
    stepper_motor_hor.rotateAngleFull(sensors::motor::Speed::HighSpeed, best_position[2]);
}

fn search_exact(
    stepper_motor_ver: StepperMotor,
    stepper_motor_hor: StepperMotor,
    interpolator_photoresistor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
    ver_left_corner: bool,
    hor_left_corner: bool,
    ver_gridsize: i32, //at least 0, only odd values
    hor_gridsize: i32, //at least 0, only odd values
    ver_init: bool,
    hor_init: bool,
) -> (i32, f32, f32, bool, bool) {
    // search for the sun within a grid around the current position
    let step_size = 1; // at least 1 // TODO calibrate
    let border = 1; // TODO calibrate
    let angle1 = stepper_motor_ver.current_angle;
    let angle2 = stepper_motor_hor.current_angle;
    let best_position = (0, angle1, angle2, true, true);
    let was_ver_border = false;
    let was_ver_border = false;

    //repeat until best position is reached
    loop {
        //move to / define one corner of the grid depending on the starting position
        if ver_init {
            let ver_offset = (ver_gridsize + 1) / 2;
            for _ in 1..ver_offset {
                angle1 = stepper_motor_ver.rotateLeftRight(sensors::motor::Speed::HighSpeed, ver_left_corner);
            }
        }
        if hor_init {
            let hor_offset = (hor_gridsize + 1) / 2;
            for _ in 1..hor_offset {
                angle2 = stepper_motor_hor.rotateLeftRight(sensors::motor::Speed::HighSpeed, hor_left_corner);
            }
        }

        //go through each position in the grid in wavy lines and check if one is better
        for m2 in 1..=hor_gridsize {
            for m1 in 1..=ver_gridsize {
                //no need to move in the first iteration as angle2 has been updated
                if m1 != 1 {
                    // depending on if we are on the left or on the right move in the opposite direction
                    let m2_odd = m2 % 2 == 0;
                    let rotate_left = (ver_left_corner && !m2_odd) || (!ver_left_corner && m2_odd);
                    for _ in 1..=step_size {
                        angle1 = stepper_motor_ver.rotateLeftRight(sensors::motor::Speed::LowSpeed, rotate_left);
                    }
                }
                photoresistor = interpolator_photoresistor.read(&mut powered_adc).unwrap();
                if best_position.0 < photoresistor {
                    let hor_border = m1_uniform <= border || m1_uniform > hor_gridsize - border;
                    let ver_border = m2 <= border || m2 > ver_gridsize - border;
                    best_position = (photoresistor, angle1, angle2, ver_border, hor_border);
                }
            }
            //if we cannot move in one direction, rotate both motors by 180°
            if (left_corner && !motor_hor.rotatableRight()) || (!left_corner && !motor_hor.rotatableLeft()){
                motor_ver.rotateAngleFull(sensors::motor::Speed::HighSpeed, (motor_ver.current_angle - 180.0).abs());
                motor_hor.rotateAngleFull(sensors::motor::Speed::HighSpeed, (motor_ver.current_angle - 180.0).abs());
            }
            angle2 = stepper_motor_hor.rotateLeftRight(sensors::motor::Speed::LowSpeed, !hor_left_corner);
        }

        //move to best position
        stepper_motor_ver.rotateAngleFull(sensors::motor::Speed::HighSpeed, best_position.1);
        stepper_motor_hor.rotateAngleFull(sensors::motor::Speed::HighSpeed, best_position.2);

        //stop if best position is not a border or we only want to execute once
        if !best_position.3 && !best_position.4  {
            break;
        } else {
            was_ver_border = was_ver_border || best_position.3;
            was_hor_border = was_hor_border || best_position.4;
        }
    }
    return (best_position.0, best_position.1, best_position.2, was_ver_border, was_hor_border);
}

fn follow_sun(stepper_motor_ver: StepperMotor,
    stepper_motor_hor: StepperMotor,
    interpolator_photoresistor: AdcInterpolator,
    powered_adc: adc::PoweredAdc,
    ver_angle_init: f32,
    hor_angle_init: f32,
    gridsize: i32,
) {
    let ver_gridsize = gridsize;
    let hor_gridsize = gridsize;
    let sleep_modifier = 0.95; //TODO calibrate
    let grid_modifier = 2; //TODO calibrate
    let min_gridsize = 3; //TODO calibrate
    let grid_angle_threshold = 3; //TODO calibrate
    let sleep = 60; //TODO calibrate
    let no_angle_move_treshold = 5; //TODO calibrate
    let light_treshold = 5; //TODO calibrate
    let zenith_reached_treshold = 0; //TODO calibrate

    //calculate the direction the sun moves horizontally and vertically
    let ver_angle = ver_angle_init;
    let hor_angle = hor_angle_init;
    while hor_angle - hor_angle_init == 0 {
        std::thread::sleep(std::time::Duration::from_secs(sleep));
        let seach_result = search_exact(stepper_motor_ver, stepper_motor_hor, interpolator_photoresistor, powered_adc, true, true, ver_gridsize, hor_gridsize, true, true);
        ver_angle = seach_result.1;
        hor_angle = search_result.2;
    }
    let ver_increase_angle = ver_angle - ver_angle_init > 0;
    let hor_increase_angle = hor_angle - hor_angle_init > 0;

    //calculate if the vertical angle does not change and the zenith may have been reached
    let zenith_reached = (ver_angle - ver_angle_init).abs <= zenith_reached_treshold;

    //repeat until sunset
    loop {
        std::thread::sleep(std::time::Duration::from_secs(sleep));
        let search_result = search_exact(stepper_motor_ver, stepper_motor_hor, interpolator_photoresistor, powered_adc, ver_increase_angle, hor_increase_angle, ver_gridsize, hor_gridsize, !zenith_reached, false);

        //sleep less long if sun was not in the first grid
        sleep = if search_result.3 || search_result.4 {
            sleep * sleep_modifier;
        } else {
            sleep / sleep_modifier;
        };

        // resize the grid vertically depending on the vertical angle change & update vertical direction & zenith boolean
        let ver_angle_move = (search_result.1 - ver_angle).abs() >= grid_angle_threshold;
        if !ver_angle_move  && ver_gridsize != min_gridsize {
            ver_gridsize = ver_gridsize - grid_modifier;
        } else if ver_angle_move && ver_gridsize != gridsize {
            ver_gridsize = ver_gridsize + grid_modifier;
        }
        zenith_reached = (ver_angle - ver_angle_init).abs <= zenith_reached_treshold;
        ver_increase_angle = ver_angle - search_result.1 > 0;
        ver_angle = seach_result.1;

        // resize the grid horizontally depending on the horizontal angle change
        let hor_angle_move = (search_result.2 - hor_angle).abs() >= grid_angle_threshold;
        if !hor_angle_move  && hor_gridsize != min_gridsize {
            hor_gridsize = hor_gridsize - grid_modifier;
        } else if hor_angle_move && hor_gridsize != gridsize {
            hor_gridsize = hor_gridsize + grid_modifier;
        }
        hor_angle = seach_result.2;

        //sunset is probably reached when angles dont change and light is low
        if(!ver_angle_move && !hor_angle_move){
            let no_angle_move = no_angle_move + 1;
            if(no_angle_move >= no_angle_move_treshold && search_result.0 < light_treshold){
                break;
            }
        }
    }
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
