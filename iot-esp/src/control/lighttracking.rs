use std::{thread::sleep, time::Duration};

use crate::sensors::motor::Speed::{High, Low};
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
    const LENGTH: usize,
> {
    stepper_motor_ver: StepperMotor<Motor1Pin1, Motor1Pin2, Motor1Pin3, Motor1Pin4>,
    stepper_motor_hor: StepperMotor<Motor2Pin1, Motor2Pin2, Motor2Pin3, Motor2Pin4>,
    interpolator_ir_sensor: AdcInterpolator<Pin1, Word, LENGTH>,
    interpolator_photoresistor: AdcInterpolator<Pin2, Word, LENGTH>,
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
        const LENGTH: usize,
    >
    Platform<
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
        LENGTH,
    >
{
    pub fn new(
        stepper_motor_ver: StepperMotor<Motor1Pin1, Motor1Pin2, Motor1Pin3, Motor1Pin4>,
        stepper_motor_hor: StepperMotor<Motor2Pin1, Motor2Pin2, Motor2Pin3, Motor2Pin4>,
        interpolator_ir_sensor: AdcInterpolator<Pin1, Word, LENGTH>,
        interpolator_photoresistor: AdcInterpolator<Pin2, Word, LENGTH>,
    ) -> Platform<
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
        LENGTH,
    > {
        Platform {
            stepper_motor_ver, stepper_motor_hor, interpolator_ir_sensor, interpolator_photoresistor
        }
    }

    pub fn init_motors<Adc, ADC>(&mut self, adc: &mut Adc) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2>,
    {
        let ir_sensor_data_close1: u32 = 0; //TODO calibrate
        let ir_sensor_data_close2: u32 = 0; //TODO calibrate

        // init stepper_motor_ver angle
        while self.read_ir(adc)? < ir_sensor_data_close1
        {
            self.stepper_motor_ver.rotate_left(Low);
        }
        self.stepper_motor_ver.stop_motor();
        self.stepper_motor_ver.init_angle(false);

        // init stepper_motor_hor angle
        //maybe use a second ir sensor
        while self.read_ir(adc)? < ir_sensor_data_close2
        {
            self.stepper_motor_hor.rotate_left(Low);
        }
        self.stepper_motor_hor.stop_motor();
        self.stepper_motor_hor.init_angle(false);
        Ok(())
    }

    pub fn search_vague<ADC, Adc>(&mut self, adc: &mut Adc) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2>,
    {
        struct BestPosition {
            photoresistor: u32,
            angle_ver: i32,
            angle_hor: i32,
        }
        //search for the sun by moving the motors in an ⧖ shape
        let mut angle_ver = self.stepper_motor_ver.current_angle();
        let mut angle_hor = self.stepper_motor_hor.current_angle();
        let mut photoresistor = self.read_photoresistor(adc)?;
        let mut best_position = BestPosition {
            photoresistor,
            angle_ver,
            angle_hor,
        };

        //1. line of the ⧖ shape
        while self.stepper_motor_hor.rotatable_right() {
            angle_hor = self.stepper_motor_hor.rotate_right(Low);
            photoresistor = self.read_photoresistor(adc)?;

            if best_position.photoresistor < photoresistor {
                best_position = BestPosition {
                    photoresistor,
                    angle_ver,
                    angle_hor,
                };
            }
        }
        self.stepper_motor_hor.stop_motor();

        //2. line of the ⧖ shape
        let half_max_angle = self.stepper_motor_ver.max_angle() / 2;
        while self.stepper_motor_ver.rotatable_angle(half_max_angle) {
            angle_ver = self
                .stepper_motor_ver
                .rotate_angle(Low, half_max_angle);
            photoresistor = self.read_photoresistor(adc)?;

            if best_position.photoresistor < photoresistor {
                best_position = BestPosition {
                    photoresistor,
                    angle_ver,
                    angle_hor,
                };
            }
        }
        self.stepper_motor_ver.stop_motor();

        //3. line of the ⧖ shape
        while self.stepper_motor_hor.rotatable_left() {
            angle_hor = self.stepper_motor_hor.rotate_left(Low);
            photoresistor = self.read_photoresistor(adc)?;

            if best_position.photoresistor < photoresistor {
                best_position = BestPosition {
                    photoresistor,
                    angle_ver,
                    angle_hor,
                };
            }
        }
        self.stepper_motor_hor.stop_motor();

        
        /* 
        let refresh_best_position =
            |angle_ver, angle_hor| -> Result<BestPosition, LightTrackingError> {
                photoresistor = self.read_photoresistor(adc)?;
                if best_position.photoresistor < photoresistor {
                    Ok(BestPosition {
                        photoresistor,
                        angle_ver,
                        angle_hor,
                    })
                } else {
                    Ok(best_position)
                }
            };
        */


        //4. line of the ⧖ shape
        while self.stepper_motor_ver.rotatable_right() {
            angle_ver = self.stepper_motor_ver.rotate_right(Low);
            photoresistor = self.read_photoresistor(adc)?;
            if best_position.photoresistor < photoresistor {
                best_position = BestPosition {
                    photoresistor,
                    angle_ver,
                    angle_hor,
                };
            }
        }
        self.stepper_motor_ver.stop_motor();

        //move to best position
        self.stepper_motor_ver
            .rotate_angle_full(High, best_position.angle_ver);
        self.stepper_motor_ver.stop_motor();
        self.stepper_motor_hor
            .rotate_angle_full(High, best_position.angle_hor);
        self.stepper_motor_hor.stop_motor();

        Ok(())
    }

    pub fn search_exact<ADC, Adc>(
        &mut self,
        adc: &mut Adc,
        ver_left_corner: bool,
        hor_left_corner: bool,
        ver_gridsize: i32, // at least 0, only odd values
        hor_gridsize: i32, // at least 0, only odd values
        ver_init: bool,
        hor_init: bool,
        limit_border: bool,
    ) -> Result<(u32, i32, i32, bool, bool), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2>,
    {
        struct BestPosition {
            photoresistor: u32,
            angle_ver: i32,
            angle_hor: i32,
            ver_border: bool,
            hor_border: bool
        }

        // search for the sun within a grid around the current position
        let step_size = 1; // at least 1 // TODO calibrate
        let border = 1; // TODO calibrate
        let mut angle_ver = self.stepper_motor_ver.current_angle();
        let mut angle_hor = self.stepper_motor_hor.current_angle();
        let mut best_position = BestPosition {photoresistor: 0, angle_ver, angle_hor, ver_border: true, hor_border: true};
        let mut was_ver_border = false;
        let mut was_hor_border = false;

        //repeat until best position is reached
        loop {
            //move to / define one corner of the grid depending on the starting position
            if ver_init {
                let ver_offset = (ver_gridsize + 1) / 2;
                for _ in 1..ver_offset {
                    angle_ver = self
                        .stepper_motor_ver
                        .rotate_left_right(High, ver_left_corner);
                }
                self.stepper_motor_ver.stop_motor();
            }
            if hor_init {
                let hor_offset = (hor_gridsize + 1) / 2;
                for _ in 1..hor_offset {
                    angle_hor = self
                        .stepper_motor_hor
                        .rotate_left_right(High, hor_left_corner);
                }
                self.stepper_motor_hor.stop_motor();
            }

            //go through each position in the grid in wavy lines and check if one is better
            for m2 in 1..=hor_gridsize {
                for mut m1 in 1..=ver_gridsize {
                    //no need to move in the first iteration as angle2 has been updated
                    let m2_odd = m2 % 2 == 0;
                    let rotate_left = (ver_left_corner && !m2_odd) || (!ver_left_corner && m2_odd);

                    if m1 != 1 {
                        // depending on if we are on the left or on the right move in the opposite direction (vertically)
                        for _ in 1..=step_size {
                            angle_ver = self
                                .stepper_motor_ver
                                .rotate_left_right(Low, rotate_left);
                        }
                    }

                    let photoresistor = self.read_ir(adc)?;

                    if best_position.photoresistor < photoresistor {
                        //update m1 so it represents the correct coordinates of the grid starting in the "left"-left corner
                        if (m2_odd && ver_left_corner) || (!m2_odd && !ver_left_corner) {
                            m1 = ver_gridsize + 1 - m1;
                        }
                        //calculate if the new best position is a border or not
                        let hor_border = if limit_border {
                            m1 <= border || m1 > hor_gridsize - border
                        } else {
                            (!ver_left_corner && m1 <= border)
                                || (ver_left_corner && m1 > ver_gridsize - border)
                        };
                        let ver_border = if limit_border {
                            m2 <= border || m2 > ver_gridsize - border
                        } else {
                            (!hor_left_corner && m2 <= border)
                                || (hor_left_corner && m2 > ver_gridsize - border)
                        };
                        best_position = BestPosition { photoresistor, angle_ver, angle_hor, ver_border, hor_border };
                    }
                }
                self.stepper_motor_ver.stop_motor();

                //if we cannot move in one direction, rotate both motors by 180°
                if (hor_left_corner && !self.stepper_motor_hor.rotatable_right())
                    || (!hor_left_corner && !self.stepper_motor_hor.rotatable_left())
                {
                    self.stepper_motor_ver.rotate_angle_full(
                        High,
                        (self.stepper_motor_ver.current_angle()
                            - 18000)
                            .abs(),
                    );
                    self.stepper_motor_ver.stop_motor();
                    self.stepper_motor_hor.rotate_angle_full(
                        High,
                        (self.stepper_motor_ver.current_angle()
                            - 18000)
                            .abs(),
                    );
                    self.stepper_motor_hor.stop_motor();
                }
                angle_hor = self
                    .stepper_motor_hor
                    .rotate_left_right(Low, !hor_left_corner);
                self.stepper_motor_hor.stop_motor();
            }

            //move to best position
            self.stepper_motor_ver
                .rotate_angle_full(High, best_position.angle_ver);
            self.stepper_motor_ver.stop_motor();
            self.stepper_motor_hor
                .rotate_angle_full(High, best_position.angle_hor);
            self.stepper_motor_hor.stop_motor();

            //stop if best position is not a border
            if !best_position.ver_border && !best_position.hor_border {
                break;
            } else {
                was_ver_border = was_ver_border || best_position.ver_border;
                was_hor_border = was_hor_border || best_position.hor_border;
            }
        }

        Ok((
            best_position.photoresistor,
            best_position.angle_ver,
            best_position.angle_hor,
            was_ver_border,
            was_hor_border,
        ))
    }

    pub fn follow_sun<ADC, Adc>(
        &mut self,
        adc: &mut Adc,
        gridsize: i32,
    ) -> Result<(), LightTrackingError>
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2>,
    {
        let mut ver_gridsize = gridsize;
        let mut hor_gridsize = gridsize;
        let sleep_modifier = 1; //TODO calibrate
        let grid_modifier = 2; //TODO calibrate
        let min_gridsize = 3; //TODO calibrate
        let grid_angle_threshold = 3; //TODO calibrate
        let mut sleep_seconds = 60; //TODO calibrate
        let no_angle_move_threshold = 5; //TODO calibrate
        let light_threshold = 5; //TODO calibrate
        let zenith_reached_threshold = 0; //TODO calibrate

        //calculate the direction the sun moves horizontally and vertically
        let ver_angle_init = self.stepper_motor_ver.current_angle();
        let hor_angle_init = self.stepper_motor_ver.current_angle();
        let mut ver_angle = ver_angle_init;
        let mut hor_angle = hor_angle_init;

        while hor_angle - hor_angle_init == 0 {
            sleep(Duration::from_secs(sleep_seconds));
            let search_result = self.search_exact(
                adc,
                true,
                true,
                ver_gridsize,
                hor_gridsize,
                true,
                true,
                false,
            )?;
            (ver_angle, hor_angle) = (search_result.1, search_result.2);
        }

        let mut ver_increase_angle = ver_angle - ver_angle_init > 0;
        let hor_increase_angle = hor_angle - hor_angle_init > 0;

        //calculate if the vertical angle does not change and the zenith may have been reached
        let mut zenith_reached = (ver_angle - ver_angle_init).abs() <= zenith_reached_threshold;

        //repeat until sunset
        let mut no_angle_move = 0;
        loop {
            sleep(Duration::from_secs(sleep_seconds));
            let search_result = self.search_exact(
                adc,
                ver_increase_angle,
                hor_increase_angle,
                ver_gridsize,
                hor_gridsize,
                zenith_reached,
                false,
                true,
            )?;

            //sleep less long if sun was not in the first grid
            sleep_seconds = if search_result.3 || search_result.4 {
                sleep_seconds * sleep_modifier
            } else {
                sleep_seconds / sleep_modifier
            };

            // resize the grid vertically depending on the vertical angle change & update vertical direction & zenith boolean
            let ver_angle_move = (search_result.1 - ver_angle).abs() >= grid_angle_threshold;
            if !ver_angle_move && ver_gridsize != min_gridsize {
                ver_gridsize -= grid_modifier;
            } else if ver_angle_move && ver_gridsize != gridsize {
                ver_gridsize += grid_modifier;
            }

            zenith_reached = (ver_angle - ver_angle_init).abs() <= zenith_reached_threshold;
            ver_increase_angle = ver_angle - search_result.1 > 0;
            ver_angle = search_result.1;

            // resize the grid horizontally depending on the horizontal angle change
            let hor_angle_move = (search_result.2 - hor_angle).abs() >= grid_angle_threshold;
            if !hor_angle_move && hor_gridsize != min_gridsize {
                hor_gridsize -= grid_modifier;
            } else if hor_angle_move && hor_gridsize != gridsize {
                hor_gridsize += grid_modifier;
            }
            hor_angle = search_result.2;

            //sunset is probably reached when angles dont change and light is low
            if !ver_angle_move && !hor_angle_move {
                no_angle_move += 1;
                if no_angle_move >= no_angle_move_threshold && search_result.0 < light_threshold {
                    break;
                }
            } else {
                no_angle_move = 0;
            }
        }

        Ok(())
    }

    fn read_ir<Adc, ADC> (&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError> 
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2>,
    {
        Ok(self
            .interpolator_ir_sensor
            .read(adc)
            .map_err(|_| LightTrackingError::ADCFailed)?
            .expect("Interpolation of infrared sensor failed"))
    }

    fn read_photoresistor<Adc, ADC> (&mut self, adc: &mut Adc) -> Result<u32, LightTrackingError> 
    where
        Word: Copy + Into<u32> + PartialEq + PartialOrd,
        Pin1: Channel<ADC>,
        Pin2: Channel<ADC>,
        Adc: OneShot<ADC, Word, Pin1> + OneShot<ADC, Word, Pin2>,
    {
        Ok(self
            .interpolator_photoresistor
            .read(adc)
            .map_err(|_| LightTrackingError::ADCFailed)?
            .expect("Interpolation of photoresistor sensor failed"))
    }

}
