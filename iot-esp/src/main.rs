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

#[derive(Clone, Copy, Debug)]
struct DataPoint {
    timestamp: std::time::SystemTime,
    temperature: f32,
    photoresitor: u32,
    ir_sensor: u32,
    voltage: u32,
    current: u32,
    power: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(i32)]
#[derive(PartialEq)]
enum CommandType {
    Nop,
    Location,
    LightTracking,
    Stop,
}

struct Command {
    comand: CommandType,
    azimuth: f32,
    altitude: f32,
}

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    //esp_idf_svc::log::EspLogger.set_target_level("rust-logging", esp_idf_svc::log::Level::Debug);

    let mut i2c_sensors =
        sensors::I2CDevices::new(peripherals.i2c0, pins.gpio21, pins.gpio22, true, true)?;

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
    let mut stepper_motor_hor = StepperMotor::new(
        pins.gpio12.into_output()?,
        pins.gpio14.into_output()?,
        pins.gpio27.into_output()?,
        pins.gpio26.into_output()?,
        480, // TODO calibrate
        1,   // TODO to be determined
        0,
        true,
    );

    /*
    loop {
        log::info!(
            "Voltage: {}, Current: {}, Power: {}, Shunt Voltage: {}",
            i2c_sensors.get_voltage(),
            i2c_sensors.get_current(),
            i2c_sensors.get_power(),
            i2c_sensors.get_shunt_voltage()
        );
        std::thread::sleep(Duration::from_secs(2));
    }
    */

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
    let interpolator_photoresistor: AdcInterpolator<Gpio34<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_photoresistor, config_photoresistor);
    let interpolator_ir_sensor_1: AdcInterpolator<Gpio35<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_ir_sensor, config_ir_sensor);
    let interpolator_button_sensor: AdcInterpolator<Gpio32<adc::Atten11dB<adc::ADC1>>, u16, 3> =
        AdcInterpolator::new(pin_button_sensor, config_button_sensor);

    let mut powered_adc = adc::PoweredAdc::new(
        peripherals.adc1,
        adc::config::Config::new().calibration(true),
    )?;

    let _wifi = networking::wifi::wifi(
        Arc::new(EspNetifStack::new()?),
        Arc::new(EspSysLoopStack::new()?),
        Arc::new(EspDefaultNvs::new()?),
    );

    let mut coap_conn = Connection::new();

    // Main motor algorithm
    let mut platform1 = Platform::new(
        stepper_motor_ver,
        stepper_motor_hor,
        interpolator_ir_sensor_1,
        interpolator_photoresistor,
        interpolator_button_sensor,
    );

    let mut command = request_command(conn, addr);
    while true {
        command = match command.command {
            CommandType::Nop =>  request_command(conn, addr),
            CommandType::Location => control_platform(coap_conn, powered_adc, platform1, i2c_sensors, command),
            CommandType::LightTracking => control_platform(coap_conn, powered_adc, platform1, i2c_sensors, command)
        }
        std::thread::sleep(Duration::from_millis(10000));
    }

    Ok(())
}

fn control_platform(coap_conn: &mut Connection, powered_adc: &mut adc::PoweredAdc, platform1: &mut Platform, i2c_sensors: &mut sensors::I2CDevices, command: &mut Command) -> Command{
    // TODO: For now the initial position at angle 0 is assumed
    // platform1.init_motors(&mut powered_adc);

    platform1.find_best_position(&mut powered_adc).unwrap();

    //calculate offset; only necessary for CommandType Location, but is not harmful for CommandType LightTracking
    let ver_angle_offset = platform1.stepper_motor_ver.current_angle() - command.altitude;
    let hor_angle_offset = platform1.stepper_motor_hor.current_angle() - command.azimuth;

    let mut datapoints = vec![];

    'outer: loop {
        let sleep_time = if command.command == CommandType::LightTracking {
            platform1.follow_light(&mut powered_adc).unwrap()
        } else {
            platform1.rotata_to_angle(command.altitude + ver_angle_offset, command.azimuth + ver_angle_offset)
            //TODO calc sleep_time similar to follow_light
            10
        };

        // Prepare datapoint to transfer
        let datapoint = DataPoint {
            timestamp: std::time::SystemTime::now(),
            temperature: i2c_sensors.get_temperature(),
            photoresitor: platform1.read_photoresistor(&mut powered_adc).unwrap(),
            ir_sensor: platform1.read_ir(&mut powered_adc).unwrap(),
            voltage: i2c_sensors.get_voltage() as u32,
            current: i2c_sensors.get_current() as u32,
            power: i2c_sensors.get_power() as u32,
        };
        datapoints.push(datapoint);

        if send_sensor_data(&mut coap_conn, "10.0.100.1:5683", &datapoints) {
            datapoints.clear();
        }

        // Sleep 10x as often but 10x less time per sleep
        for _ in 0..sleep_time * 10 {
            if platform1.reset_if_button_pressed(&mut powered_adc) {
                break 'outer;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        let new_command = request_command(conn, addr);
        if command.command == new_command.command {
            command = new_command; //override azimuth and altitude
        } else if command.command != CommandType::Nop {
            command = new_command; //set return value
            break 'outer;
        }
    }

    // Motor stopped, now only try to delivery datapoints
    while !datapoints.is_empty() {
        if send_sensor_data(&mut coap_conn, "10.0.100.1:5683", &datapoints) {
            datapoints.clear();
        }
    }

    command
}


fn request_command(conn: &mut Connection, addr: &str) -> Command {
    match conn.request(RequestType::GET, addr, "/command", Vec::new()) {
        Ok(x) => {
            let (command_bytes, payload1) = payload.split_at(std::mem::size_of::<i32>());
            let command: CommandType = unsafe { ::std::mem::transmute(i32::from_le_bytes(command_bytes.try_into().unwrap()))};

            let (azimuth_bytes, payload2) = payload1.split_at(std::mem::size_of::<f32>());
            let azimuth = f32::from_le_bytes(azimuth_bytes.try_into().unwrap());

            let (altitude_bytes, payload3) = payload2.split_at(std::mem::size_of::<f32>());
            let altitude = f32::from_le_bytes(altitude_bytes.try_into().unwrap());

            debug_assert_eq!(0, payload3.len());

            Command {command, azimuth, altitude}
        },
        Err(e) => {
            log::error!("{:?}", e);
            Command {CommandType::Nop, 0, 0}
        }
    }
}

fn send_sensor_data(conn: &mut Connection, addr: &str, datapoints: &[DataPoint]) -> bool {
    // length_s + timestamp_s + datasets_length * (timestamp + temperature + photoresistor + IRsensor + voltage + current + power)
    let mut payload = vec![0; 4 + 8 + datapoints.len() * (8 + 4 * 6)];

    let mut index = 0;
    // 4 bytes: Amount of datasets in payload
    payload[index..index + 4].copy_from_slice(&(datapoints.len() as u32).to_le_bytes());
    index += 4;
    // 8 bytes: Placeholder for current SystemTime
    payload[index..index + 8].copy_from_slice(&0u64.to_le_bytes());
    index += 8;

    // TODO: prevent fragementation
    for datapoint in datapoints {
        let unix_time = datapoint
            .timestamp
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        payload[index..index + 8].copy_from_slice(&unix_time.to_le_bytes());
        index += 8;
        payload[index..index + 4].copy_from_slice(&datapoint.temperature.to_le_bytes());
        index += 4;
        payload[index..index + 4].copy_from_slice(&datapoint.photoresitor.to_le_bytes());
        index += 4;
        payload[index..index + 4].copy_from_slice(&datapoint.ir_sensor.to_le_bytes());
        index += 4;
        payload[index..index + 4].copy_from_slice(&datapoint.voltage.to_le_bytes());
        index += 4;
        payload[index..index + 4].copy_from_slice(&datapoint.current.to_le_bytes());
        index += 4;
        payload[index..index + 4].copy_from_slice(&datapoint.power.to_le_bytes());
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
        Ok(_) => true,
        Err(e) => {
            log::error!("{:?}", e);
            false
        }
    }
}
