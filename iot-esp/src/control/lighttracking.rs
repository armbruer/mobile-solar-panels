use std::cmp::Ordering;
use std::ops::{Add, Sub};

use crate::sensors::motor::Speed;
use crate::sensors::motor::StepperMotor;
use adc_interpolator::AdcInterpolator;
use embedded_hal::{
    adc::{Channel, OneShot},
    digital::v2::OutputPin,
};

#[derive(Clone, Copy, Debug)]
pub enum LightTrackingError {
    ADCFailed,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MotorAngles {
    pub motor_hor: i32,
    pub motor_ver: i32,
}

impl Add<&MotorAngles> for &MotorAngles {
    type Output = MotorAngles;

    fn add(self, rhs: &MotorAngles) -> Self::Output {
        MotorAngles {
            motor_hor: self.motor_hor + rhs.motor_hor,
            motor_ver: self.motor_ver + rhs.motor_ver,
        }
    }
}

impl Sub<&MotorAngles> for &MotorAngles {
    type Output = MotorAngles;

    fn sub(self, rhs: &MotorAngles) -> Self::Output {
        MotorAngles {
            motor_hor: self.motor_hor - rhs.motor_hor,
            motor_ver: self.motor_ver - rhs.motor_ver,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    None,
    Left,
    Right,
}

pub trait PlatformTrait<
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
>
{
    fn new(
        stepper_motor_ver: StepperMotor<Motor1Pin1, Motor1Pin2, Motor1Pin3, Motor1Pin4>,
        stepper_motor_hor: StepperMotor<Motor2Pin1, Motor2Pin2, Motor2Pin3, Motor2Pin4>,
        interpolator_ir_sensor: AdcInterpolator<Pin1, Word, LENGTH>,
        interpolator_photoresistor: AdcInterpolator<Pin2, Word, LENGTH>,
        interpolator_button: AdcInterpolator<Pin3, Word, LENGTH>,
    ) -> Self;

    fn reset_motors_position(&mut self);

    fn reset_if_button_pressed<Adc, ADC>(&mut self, adc: &mut Adc) -> bool
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>;

    fn get_current_angles(&self) -> MotorAngles;

    fn test_movement(&mut self);

    fn rotate_to_angle(&mut self, ver_angle: i32, hor_angle: i32);

    fn init_motors<Adc, ADC>(&mut self, adc: &mut Adc) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>;

    fn find_best_position<ADC, Adc>(&mut self, adc: &mut Adc) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>;

    fn search_scope<ADC, Adc>(
        &mut self,
        adc: &mut Adc,
        speed: Speed,
        angle_hor: i32,
        angle_ver: i32,
    ) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>;

    fn follow_light<ADC, Adc>(&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>;

    fn read_ir<Adc, ADC>(&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>;

    fn read_photoresistor<Adc, ADC>(&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>;
}

pub struct Platform<
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
    const LENGTH: usize,
> {
    stepper_motor_ver: StepperMotor<Motor1Pin1, Motor1Pin2, Motor1Pin3, Motor1Pin4>,
    stepper_motor_hor: StepperMotor<Motor2Pin1, Motor2Pin2, Motor2Pin3, Motor2Pin4>,
    interpolator_ir_sensor: AdcInterpolator<Pin1, Word, LENGTH>,
    interpolator_photoresistor: AdcInterpolator<Pin2, Word, LENGTH>,
    interpolator_button: AdcInterpolator<Pin3, Word, LENGTH>,

    last_angle_hor: i32,
    last_angle_ver: i32,
    hor_direction: Direction,
}

impl<
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
    >
    PlatformTrait<
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
    >
    for Platform<
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
    >
{
    fn new(
        stepper_motor_ver: StepperMotor<Motor1Pin1, Motor1Pin2, Motor1Pin3, Motor1Pin4>,
        stepper_motor_hor: StepperMotor<Motor2Pin1, Motor2Pin2, Motor2Pin3, Motor2Pin4>,
        interpolator_ir_sensor: AdcInterpolator<Pin1, Word, LENGTH>,
        interpolator_photoresistor: AdcInterpolator<Pin2, Word, LENGTH>,
        interpolator_button: AdcInterpolator<Pin3, Word, LENGTH>,
    ) -> Self {
        Platform {
            stepper_motor_ver,
            stepper_motor_hor,
            interpolator_ir_sensor,
            interpolator_photoresistor,
            interpolator_button,
            last_angle_hor: 0,
            last_angle_ver: 0,
            hor_direction: Direction::None,
        }
    }

    fn reset_motors_position(&mut self) {
        self.stepper_motor_hor.rotate_to_angle(Speed::High, 0);
        self.stepper_motor_hor.stop_motor();
        self.stepper_motor_ver.rotate_to_angle(Speed::High, 0);
        self.stepper_motor_ver.stop_motor();
        self.hor_direction = Direction::None;
    }

    fn reset_if_button_pressed<Adc, ADC>(&mut self, adc: &mut Adc) -> bool
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    {
        let value = self
            .interpolator_button
            .read(adc)
            .map_err(|_| LightTrackingError::ADCFailed)
            .unwrap()
            .expect("Interpolation of infrared sensor failed");

        if value < 1500 {
            self.reset_motors_position();
            true
        } else {
            false
        }
    }

    fn get_current_angles(&self) -> MotorAngles {
        MotorAngles {
            motor_hor: self.stepper_motor_hor.current_angle(),
            motor_ver: self.stepper_motor_ver.current_angle(),
        }
    }

    fn test_movement(&mut self) {
        let mut current_angle = 0;

        let test_horizontal = true;
        if test_horizontal {
            for _ in 0..1000 {
                current_angle = self.stepper_motor_hor.rotate_left(Speed::Low);
            }
            log::info!("Rotate horizontal left stopped at angle {}", current_angle);
            for _ in 0..1000 {
                current_angle = self.stepper_motor_hor.rotate_right(Speed::Low);
            }
            log::info!("Rotate horizontal right stopped at angle {}", current_angle);
            self.stepper_motor_hor.stop_motor();
        }

        let test_vertical = true;
        if test_vertical {
            for _ in 0..1000 {
                current_angle = self.stepper_motor_ver.rotate_left(Speed::Low);
            }
            log::info!("Rotate vertical left stopped at angle {}", current_angle);
            for _ in 0..1000 {
                current_angle = self.stepper_motor_ver.rotate_right(Speed::Low);
            }
            log::info!("Rotate vertical right stopped at angle {}", current_angle);
            self.stepper_motor_ver.stop_motor();
        }
    }

    fn rotate_to_angle(&mut self, ver_angle: i32, hor_angle: i32) {
        self.stepper_motor_ver
            .rotate_to_angle(Speed::High, ver_angle);
        self.stepper_motor_ver.stop_motor();
        self.stepper_motor_hor
            .rotate_to_angle(Speed::High, hor_angle);
        self.stepper_motor_hor.stop_motor();
    }

    fn init_motors<Adc, ADC>(&mut self, adc: &mut Adc) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    {
        log::info!("Initiating motors position");

        let ir_sensor_data_close: u32 = 1500;

        /*
        // init stepper_motor_ver angle
        log::info!("Rotating vertical until IR sensor hits");
        while self.read_ir(adc)? > ir_sensor_data_close {
            self.stepper_motor_ver.rotate_right(Speed::Low);
        }
        self.stepper_motor_ver.stop_motor();
        self.stepper_motor_ver.init_angle();
        */

        // init stepper_motor_hor angle
        log::info!("Rotating horizontal until IR sensor hits");
        while self.read_ir(adc)? > ir_sensor_data_close {
            self.stepper_motor_hor.rotate_right(Speed::Low);
        }
        self.stepper_motor_hor.stop_motor();
        self.stepper_motor_hor.init_angle();

        log::info!("Initiating motors finished");
        Ok(())
    }

    fn find_best_position<ADC, Adc>(&mut self, adc: &mut Adc) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    {
        self.reset_motors_position();

        //search for the sun by moving the motors
        let mut best_photoresistor = self.read_photoresistor(adc)?;
        let mut best_angle_hor = self.stepper_motor_hor.current_angle();
        let mut best_angle_ver = self.stepper_motor_ver.current_angle();

        while self.stepper_motor_hor.rotatable_left() {
            let angle_hor = self.stepper_motor_hor.rotate_left(Speed::HighMedium);
            let photoresistor = self.read_photoresistor(adc)?;

            if best_photoresistor > photoresistor {
                best_photoresistor = photoresistor;
                best_angle_hor = angle_hor;
            }
        }
        log::info!("Found best horizontal light at {}", best_angle_hor);
        // Move to best horizontal position
        self.stepper_motor_hor
            .rotate_to_angle(Speed::High, best_angle_hor);
        self.stepper_motor_hor.stop_motor();

        let half_max_angle = self.stepper_motor_ver.max_angle() / 2;
        while self.stepper_motor_ver.rotatable_to_angle(half_max_angle) {
            let angle_ver = self
                .stepper_motor_ver
                .rotate_single_step_to_angle(Speed::HighMedium, half_max_angle);
            let photoresistor = self.read_photoresistor(adc)?;

            if best_photoresistor > photoresistor {
                best_photoresistor = photoresistor;
                best_angle_ver = angle_ver;
            }
        }
        log::info!("Found best vertical light at {}", best_angle_ver);
        // Move to best vertical position
        self.stepper_motor_ver
            .rotate_to_angle(Speed::High, best_angle_ver);
        self.stepper_motor_ver.stop_motor();

        Ok(())
    }

    fn search_scope<ADC, Adc>(
        &mut self,
        adc: &mut Adc,
        speed: Speed,
        angle_hor: i32,
        angle_ver: i32,
    ) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    {
        let init_angle_hor = self.stepper_motor_hor.current_angle();
        let init_angle_ver = self.stepper_motor_ver.current_angle();

        //search for the sun by moving the motors
        let mut best_photoresistor = self.read_photoresistor(adc)?;
        let mut best_angle_hor = self.stepper_motor_hor.current_angle();
        let mut best_angle_ver = self.stepper_motor_ver.current_angle();

        let search_range = match self.hor_direction {
            Direction::None => {
                self.stepper_motor_hor
                    .rotate_to_angle(Speed::HighMedium, init_angle_hor - angle_hor / 2);
                (init_angle_hor - angle_hor / 2)..(init_angle_hor + angle_hor / 2)
            }
            // if we only search in one direction we can skip half of the search
            Direction::Left => init_angle_hor..(init_angle_hor + angle_hor / 2),
            Direction::Right => (init_angle_hor - angle_hor / 2)..init_angle_hor,
        };

        for angle in search_range {
            let angle_hor = self.stepper_motor_hor.rotate_to_angle(speed, angle);
            let photoresistor = self.read_photoresistor(adc)?;

            if best_photoresistor > photoresistor {
                best_photoresistor = photoresistor;
                best_angle_hor = angle_hor;
            }
        }
        log::info!("Found best horizontal light at {}", best_angle_hor);
        // Move to best horizontal position
        self.stepper_motor_hor
            .rotate_to_angle(Speed::HighMedium, best_angle_hor);
        self.stepper_motor_hor.stop_motor();

        self.hor_direction = match best_angle_hor.cmp(&init_angle_hor) {
            Ordering::Greater => Direction::Left,
            Ordering::Equal => Direction::None,
            Ordering::Less => Direction::Right,
        };

        self.stepper_motor_ver
            .rotate_to_angle(Speed::HighMedium, init_angle_ver - angle_ver / 2);
        for angle in (init_angle_ver - angle_ver / 2)..(init_angle_ver + angle_ver / 2) {
            let angle_ver = self.stepper_motor_ver.rotate_to_angle(speed, angle);
            let photoresistor = self.read_photoresistor(adc)?;

            if best_photoresistor > photoresistor {
                best_photoresistor = photoresistor;
                best_angle_ver = angle_ver;
            }
        }
        log::info!("Found best vertical light at {}", best_angle_ver);
        // Move to best vertical position
        self.stepper_motor_ver
            .rotate_to_angle(Speed::HighMedium, best_angle_ver);
        self.stepper_motor_ver.stop_motor();

        Ok(())
    }

    fn follow_light<ADC, Adc>(&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    {
        self.search_scope(adc, Speed::Low, 80, 40)?;

        let new_angle_hor = self.stepper_motor_hor.current_angle();
        let new_angle_ver = self.stepper_motor_ver.current_angle();

        let sleep_time_hor = if new_angle_hor.abs_diff(self.last_angle_hor) > 30 {
            2
        } else if new_angle_hor.abs_diff(self.last_angle_hor) > 2 {
            5
        } else {
            15
        };

        let sleep_time_ver = if new_angle_ver.abs_diff(self.last_angle_ver) > 15 {
            2
        } else if new_angle_ver.abs_diff(self.last_angle_ver) > 2 {
            5
        } else {
            15
        };

        self.last_angle_hor = new_angle_hor;
        self.last_angle_ver = new_angle_ver;

        Ok(sleep_time_hor.max(sleep_time_ver))
    }

    fn read_ir<Adc, ADC>(&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    {
        Ok(self
            .interpolator_ir_sensor
            .read(adc)
            .map_err(|_| LightTrackingError::ADCFailed)?
            .expect("Interpolation of infrared sensor failed"))
    }

    fn read_photoresistor<Adc, ADC>(&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Pin3: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2> + OneShot<ADC, Word, Pin3>,
    {
        Ok(self
            .interpolator_photoresistor
            .read(adc)
            .map_err(|_| LightTrackingError::ADCFailed)?
            .expect("Interpolation of photoresistor sensor failed"))
    }
}
