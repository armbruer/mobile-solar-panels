mod control;
mod networking;
mod sensors;

use std::convert::TryInto;
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

use adc_interpolator::AdcInterpolator;
use coap_lite::RequestType;
use control::lighttracking::{MotorAngles, PlatformTrait};
use embedded_hal::adc::{Channel, OneShot};
use embedded_hal::digital::v2::OutputPin;
use esp_idf_hal::adc;
use esp_idf_hal::gpio::{Gpio32, Gpio34, Gpio35};
use esp_idf_hal::prelude::Peripherals;

use esp_idf_svc::netif::EspNetifStack;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::sysloop::EspSysLoopStack;
use esp_idf_sys::EspError;
use esp_idf_sys::{self as _}; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use networking::coap::Connection;
use num_enum::TryFromPrimitive;
use sensors::motor::{Speed, StepperMotor};

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

#[derive(Clone, Copy, Debug, TryFromPrimitive, PartialEq)]
#[repr(u8)]
enum CommandType {
    Nop,
    Location,
    LightTracking,
    Follower,
    Stop,
}

impl Default for CommandType {
    fn default() -> Self {
        Self::Nop
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Command {
    command: CommandType,
    target_angle_offset_hor: i32,
    target_angle_offset_ver: i32,
    azimuth: f32,
    altitude: f32,
}

// 540 steps = 360°
const FULL_ROTATION_ANGLE: i32 = 540;

fn convert_azimuth_altitude(azimuth: f32, altitude: f32) -> (i32, i32) {
    (
        ((-azimuth + 2.0 * PI) / (2.0 * PI) * FULL_ROTATION_ANGLE as f32) as i32,
        (altitude / (2.0 * PI) * FULL_ROTATION_ANGLE as f32) as i32,
    )
}

fn main() -> Result<(), EspError> {
    let device_id: u32 = env!("esp_device_id").parse().unwrap();

    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    //esp_idf_svc::log::EspLogger.set_target_level("rust-logging", esp_idf_svc::log::Level::Debug);

    let mut i2c_sensors =
        sensors::I2CDevices::new(peripherals.i2c0, pins.gpio21, pins.gpio22, true, true)?;

    let stepper_motor_ver = StepperMotor::new(
        pins.gpio16.into_output()?,
        pins.gpio17.into_output()?,
        pins.gpio18.into_output()?,
        pins.gpio19.into_output()?,
        (FULL_ROTATION_ANGLE as f32 / 3.5) as i32,
        1,
        0,
        true,
    );

    let stepper_motor_hor = StepperMotor::new(
        pins.gpio26.into_output()?,
        pins.gpio27.into_output()?,
        pins.gpio14.into_output()?,
        pins.gpio12.into_output()?,
        (FULL_ROTATION_ANGLE as f32 / 1.6) as i32,
        1,
        1,
        false,
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

    // Main motor algorithm
    let mut platform1 = control::lighttracking::Platform::new(
        stepper_motor_ver,
        stepper_motor_hor,
        interpolator_ir_sensor_1,
        interpolator_photoresistor,
        interpolator_button_sensor,
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

    /*
    platform1.test_movement();
    return Ok(());
    */

    let _wifi = networking::wifi::wifi(
        Arc::new(EspNetifStack::new()?),
        Arc::new(EspSysLoopStack::new()?),
        Arc::new(EspDefaultNvs::new()?),
    );

    platform1.init_motors(&mut powered_adc).unwrap();

    let mut coap_conn = Connection::new();

    let addr = "10.0.100.1:5683";

    let mut datapoints = vec![];

    // TODO: Poll some time for edge and then start with default mode
    let mut command = Command::default();
    let mut world_angles_offset = MotorAngles::default();
    let mut initial_platform_offset = MotorAngles::default();

    'stop_loop: loop {
        'main_loop: loop {
            // Replace command only if received a new command that is not NOP
            let new_command = match request_command(
                &mut coap_conn,
                addr,
                &(&platform1.get_current_angles() - &initial_platform_offset),
                device_id,
            ) {
                Some(cmd) => cmd,
                None => command,
            };

            if new_command.command != command.command {
                // Received instruction to change command
                // Init the platform for the new command
                match new_command.command {
                    CommandType::Nop => (),
                    CommandType::Follower | CommandType::LightTracking => {
                        platform1.find_best_position(&mut powered_adc).unwrap();
                        initial_platform_offset = platform1.get_current_angles();
                    }
                    CommandType::Location => {
                        platform1.find_best_position(&mut powered_adc).unwrap();
                        initial_platform_offset = platform1.get_current_angles();

                        world_angles_offset = platform1.get_current_angles();
                        let (angle_offset_hor, angle_offset_ver) =
                            convert_azimuth_altitude(new_command.azimuth, new_command.altitude);
                        world_angles_offset.motor_hor -= angle_offset_hor;
                        world_angles_offset.motor_ver -= angle_offset_ver;
                    }
                    CommandType::Stop => break 'main_loop,
                }
            }

            if new_command.command != CommandType::Nop {
                command = new_command;
            }

            // Platform is initialized for the command, now execute them
            let sleep_time = match command.command {
                CommandType::Nop => 10,
                CommandType::Follower | CommandType::Location | CommandType::LightTracking => {
                    control_platform(
                        &mut powered_adc,
                        &mut platform1,
                        &command,
                        &world_angles_offset,
                        &initial_platform_offset,
                    )
                }
                CommandType::Stop => panic!("Requested to execute stop"),
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
            log::debug!("Adding {:?}", &datapoint);
            datapoints.push(datapoint);

            if send_sensor_data(&mut coap_conn, addr, &datapoints, device_id) {
                datapoints.clear();
            }

            // Sleep 10x as often but 10x less time per sleep
            for _ in 0..sleep_time * 10 {
                if platform1.reset_if_button_pressed(&mut powered_adc) {
                    break 'stop_loop;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        platform1.reset_motors_position();

        if platform1.reset_if_button_pressed(&mut powered_adc) {
            break 'stop_loop;
        }
        std::thread::sleep(Duration::from_millis(10000));

        // Motor stopped, now only try to delivery datapoints
        while !datapoints.is_empty() {
            if send_sensor_data(&mut coap_conn, addr, &datapoints, device_id) {
                datapoints.clear();
            }
        }
    }

    Ok(())
}

fn control_platform<
    T,
    Motor1Pin1: OutputPin,
    Motor1Pin2: OutputPin,
    Motor1Pin3: OutputPin,
    Motor1Pin4: OutputPin,
    Motor2Pin1: OutputPin,
    Motor2Pin2: OutputPin,
    Motor2Pin3: OutputPin,
    Motor2Pin4: OutputPin,
    Word: Copy + Into<u32> + PartialEq + PartialOrd,
    Pin1,
    Pin2,
    Pin3,
    const LENGTH: usize,
    ADC,
    Adc,
>(
    adc: &mut Adc,
    platform1: &mut T,
    command: &Command,
    world_angles_offset: &MotorAngles,
    initial_platform_offset: &MotorAngles,
) -> u32
where
    Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    Pin1: Channel<ADC>,
    Pin2: Channel<ADC>,
    Pin3: Channel<ADC>,

    T: PlatformTrait<
        Motor1Pin1,
        Motor1Pin2,
        Motor1Pin3,
        Motor1Pin4,
        Motor2Pin1,
        Motor2Pin2,
        Motor2Pin3,
        Motor2Pin4,
        Word,
        Pin1,
        Pin2,
        Pin3,
        LENGTH,
    >,
{
    match command.command {
        CommandType::Nop | CommandType::Stop => {
            panic!("Invalid CommandType in control_platform(): {:?}", command)
        }
        CommandType::Follower => {
            platform1.rotate_to_angle(
                initial_platform_offset.motor_ver + command.target_angle_offset_ver,
                initial_platform_offset.motor_hor + command.target_angle_offset_hor,
                Speed::Medium,
            );
            10
        }
        CommandType::LightTracking => platform1.follow_light(adc).unwrap(),
        CommandType::Location => {
            let (angle_hor, angle_ver) =
                convert_azimuth_altitude(command.azimuth, command.altitude);
            platform1.rotate_to_angle(
                angle_ver + world_angles_offset.motor_ver,
                angle_hor + world_angles_offset.motor_hor,
                Speed::Medium,
            );
            // TODO: calc sleep_time similar to follow_light
            10
        }
    }
}

fn request_command(
    conn: &mut Connection,
    addr: &str,
    target_angle_offset: &MotorAngles,
    device_id: u32,
) -> Option<Command> {
    let mut payload = vec![0; 12];
    payload[0..4].copy_from_slice(&device_id.to_le_bytes());
    payload[4..8].copy_from_slice(&target_angle_offset.motor_hor.to_le_bytes());
    payload[8..12].copy_from_slice(&target_angle_offset.motor_ver.to_le_bytes());
    match conn.request(RequestType::Get, addr, "/command", payload) {
        Ok(response) => {
            let mut payload_rest;

            let command_bytes;
            (command_bytes, payload_rest) =
                response.message.payload.split_at(std::mem::size_of::<u8>());
            let command = u8::from_le_bytes(command_bytes.try_into().unwrap())
                .try_into()
                .unwrap();

            let mut target_angle_offset_hor = 0;
            let mut target_angle_offset_ver = 0;

            let mut azimuth = 0.0;
            let mut altitude = 0.0;

            if command == CommandType::Follower {
                let target_angle_hor_bytes;
                (target_angle_hor_bytes, payload_rest) =
                    payload_rest.split_at(std::mem::size_of::<i32>());
                target_angle_offset_hor =
                    i32::from_le_bytes(target_angle_hor_bytes.try_into().unwrap());

                let target_angle_ver_bytes;
                (target_angle_ver_bytes, payload_rest) =
                    payload_rest.split_at(std::mem::size_of::<i32>());
                target_angle_offset_ver =
                    i32::from_le_bytes(target_angle_ver_bytes.try_into().unwrap());
            } else if command == CommandType::Location {
                let azimuth_bytes;
                (azimuth_bytes, payload_rest) = payload_rest.split_at(std::mem::size_of::<f32>());
                azimuth = f32::from_le_bytes(azimuth_bytes.try_into().unwrap());

                let altitude_bytes;
                (altitude_bytes, payload_rest) = payload_rest.split_at(std::mem::size_of::<f32>());
                altitude = f32::from_le_bytes(altitude_bytes.try_into().unwrap());
            }

            debug_assert_eq!(0, payload_rest.len());

            let res = Command {
                command,
                target_angle_offset_hor,
                target_angle_offset_ver,
                azimuth,
                altitude,
            };

            log::info!("request_command(): Got command: {:?}", res);

            Some(res)
        }
        Err(e) => {
            log::warn!("request_command(): {:?}", e);
            None
        }
    }
}

fn send_sensor_data(
    conn: &mut Connection,
    addr: &str,
    datapoints: &[DataPoint],
    device_id: u32,
) -> bool {
    // length_s + timestamp_s + datasets_length * (device_id + timestamp + temperature + photoresistor + IRsensor + voltage + current + power)
    let mut payload = vec![0; 4 + 8 + datapoints.len() * (4 + 8 + 4 * 6)];

    let mut index = 0;
    // 4 bytes: Amount of datasets in payload
    payload[index..index + 4].copy_from_slice(&(datapoints.len() as u32).to_le_bytes());
    index += 4;
    // 8 bytes: Placeholder for current SystemTime
    payload[index..index + 8].copy_from_slice(&0u64.to_le_bytes());
    index += 8;

    // TODO: prevent fragementation
    for datapoint in datapoints {
        payload[index..index + 4].copy_from_slice(&device_id.to_le_bytes());
        index += 4;
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
        Ok(_) => {
            log::info!("send_sensor_data(): Sent {} datapoints", datapoints.len());
            true
        }
        Err(e) => {
            log::warn!("send_sensor_data(): {:?}", e);
            false
        }
    }
}
