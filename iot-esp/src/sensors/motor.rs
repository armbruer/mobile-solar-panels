use embedded_hal::digital::v2::PinState;
use embedded_hal::digital::v2::OutputPin;

use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub enum Speed {
    // max: 16000
    LowSpeed = 10000,
    HighSpeed = 1000,
    __Stop = 0, // Internal only
}

pub struct StepperMotor<
    OutputPin1, OutputPin2, OutputPin3, OutputPin4: OutputPin,
> {
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
            current_angle: 0,
            initalized_angles: false,
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

    pub fn init_angle(&mut self, ismax_angle: bool) {
        self.current_angle = if ismax_angle { self.max_angle } else { 0 };
        self.initalized_angles = true;
    }

    pub fn rotatable_angle(&mut self, angle: i32) -> bool {
        if !self.initalized_angles
            || self.current_angle == angle
            || angle < 0
            || angle > self.max_angle
        {
            return false;
        }
        if self.current_angle < angle {
            return (self.current_angle + self.step_size) < angle;
        } else {
            return (self.current_angle - self.step_size) > angle;
        }
    }

    pub fn rotatable_right(&mut self) -> bool {
        return self.rotatable_angle(self.max_angle);
    }

    pub fn rotatable_left(&mut self) -> bool {
        return self.rotatable_angle(0);
    }

    pub fn rotate_angle_full(&mut self, motorSpeed: Speed, angle: i32) {
        while !self.rotatable_angle(angle) {
            self.rotate_angle(motorSpeed, angle);
        }
    }

    pub fn rotate_angle(&mut self, motorSpeed: Speed, angle: i32) -> i32 {
        if !self.rotatable_angle(angle) {
            return self.current_angle;
        }
        if self.current_angle < angle {
            return self.rotateRight(motorSpeed);
        } else {
            return self.rotateLeft(motorSpeed);
        }
    }

    pub fn rotateLeftRight(&mut self, motorSpeed: Speed, left: bool) -> i32 {
        if left {
            return self.rotateLeft(motorSpeed);
        } else {
            return self.rotateRight(motorSpeed);
        }
    }

    pub fn rotateRight(&mut self, motorSpeed: Speed) -> i32 {
        if !self.rotatable_right() {
            return self.current_angle;
        }

        self.setMotor(
            PinState::Low,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::High,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::High,
            PinState::High,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::High,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motorSpeed,
        );
        self.current_angle = self.current_angle + self.step_size;
        self.current_angle
    }

    pub fn rotateLeft(&mut self, motorSpeed: Speed) -> i32 {
        if !self.rotatable_left() {
            return self.current_angle;
        }

        self.setMotor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::High,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::High,
            PinState::Low,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::High,
            PinState::High,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::Low,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::Low,
            PinState::High,
            PinState::High,
            motorSpeed,
        );
        self.setMotor(
            PinState::Low,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motorSpeed,
        );
        self.setMotor(
            PinState::High,
            PinState::Low,
            PinState::Low,
            PinState::High,
            motorSpeed,
        );
        self.current_angle = self.current_angle - self.step_size;
        self.current_angle
    }

    pub fn stopMotor(&mut self) {
        self.setMotor(
            PinState::Low,
            PinState::Low,
            PinState::Low,
            PinState::Low,
            Speed::__Stop,
        );
    }

    pub fn setMotor(
        &mut self,
        in1: PinState,
        in2: PinState,
        in3: PinState,
        in4: PinState,
        motorSpeed: Speed,
    ) {
        self.pin1.set_state(in1);
        self.pin2.set_state(in2);
        self.pin3.set_state(in3);
        self.pin4.set_state(in4);
        thread::sleep(Duration::from_micros(motorSpeed as u64));
    }
}
