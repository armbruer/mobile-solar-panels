use embedded_hal::digital::v2::OutputPin;
use embedded_hal::digital::v2::PinState;

use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub enum Speed {
    // max: 16000
    Low = 10000,
    LowMedium = 8000,
    Medium = 5000,
    HighMedium = 2000,
    High = 1000,
    __Stop = 0, // Internal only
}

pub struct StepperMotor<OutputPin1, OutputPin2, OutputPin3, OutputPin4> {
    pin1: OutputPin1,
    pin2: OutputPin2,
    pin3: OutputPin3,
    pin4: OutputPin4,
    max_angle: i32,
    step_size: i32,
    current_angle: i32,
    initalized_angles: bool,
}

impl<
        OutputPin1: OutputPin,
        OutputPin2: OutputPin,
        OutputPin3: OutputPin,
        OutputPin4: OutputPin,
    > StepperMotor<OutputPin1, OutputPin2, OutputPin3, OutputPin4>
{
    pub fn new(
        pin1: OutputPin1,
        pin2: OutputPin2,
        pin3: OutputPin3,
        pin4: OutputPin4,
        max_angle: i32,
        step_size: i32,
        current_angle: i32,
        initalized_angles: bool,
    ) -> StepperMotor<OutputPin1, OutputPin2, OutputPin3, OutputPin4> {
        StepperMotor {
            pin1,
            pin2,
            pin3,
            pin4,
            max_angle,
            step_size,
            current_angle,
            initalized_angles,
        }
    }

    pub fn max_angle(&self) -> i32 {
        self.max_angle
    }

    pub fn step_size(&self) -> i32 {
        self.step_size
    }

    pub fn current_angle(&self) -> i32 {
        self.current_angle
    }

    /// Motor is in initial position at angle 0
    pub fn init_angle(&mut self) {
        self.current_angle = 0;
        self.initalized_angles = true;
    }

    pub fn rotatable_to_angle(&mut self, angle: i32) -> bool {
        if angle < 0 || angle > self.max_angle {
            return false;
        }
        if self.current_angle < angle {
            (self.current_angle + self.step_size) < angle
        } else {
            (self.current_angle - self.step_size) > angle
        }
    }

    pub fn rotatable_right(&mut self) -> bool {
        self.rotatable_to_angle(0)
    }

    pub fn rotatable_left(&mut self) -> bool {
        self.rotatable_to_angle(self.max_angle)
    }

    pub fn rotate_to_angle(&mut self, motor_speed: Speed, angle: i32) -> i32 {
        while self.rotatable_to_angle(angle) {
            self.rotate_single_step_to_angle(motor_speed, angle);
        }
        angle
    }

    pub fn rotate_single_step_to_angle(&mut self, motor_speed: Speed, angle: i32) -> i32 {
        if !self.rotatable_to_angle(angle) {
            return self.current_angle;
        }
        if self.current_angle > angle {
            self.rotate_right(motor_speed)
        } else {
            self.rotate_left(motor_speed)
        }
    }

    #[inline]
    pub fn rotate_left_right(&mut self, motor_speed: Speed, left: bool) -> i32 {
        if left {
            self.rotate_left(motor_speed)
        } else {
            self.rotate_right(motor_speed)
        }
    }

    pub fn rotate_right(&mut self, motor_speed: Speed) -> i32 {
        if !self.rotatable_right() {
            return self.current_angle;
        }

        self.set_motor(
            PinState::Low,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::High,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::High,
            PinState::High,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::High,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motor_speed,
        );
        if self.initalized_angles {
            self.current_angle -= self.step_size;
        }
        self.current_angle
    }

    pub fn rotate_left(&mut self, motor_speed: Speed) -> i32 {
        if !self.rotatable_left() {
            return self.current_angle;
        }

        self.set_motor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::High,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::High,
            PinState::High,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::Low,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::High,
            motor_speed,
        );
        self.set_motor(
            PinState::Low,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motor_speed,
        );
        self.set_motor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motor_speed,
        );
        if self.initalized_angles {
            self.current_angle += self.step_size;
        }
        self.current_angle
    }

    pub fn stop_motor(&mut self) {
        self.set_motor(
            PinState::Low,
            PinState::Low,
            PinState::Low,
            PinState::Low,
            Speed::__Stop,
        );
    }

    pub fn set_motor(
        &mut self,
        in1: PinState,
        in2: PinState,
        in3: PinState,
        in4: PinState,
        motor_speed: Speed,
    ) {
        self.pin1.set_state(in1);
        self.pin2.set_state(in2);
        self.pin3.set_state(in3);
        self.pin4.set_state(in4);
        thread::sleep(Duration::from_micros(motor_speed as u64));
    }
}
