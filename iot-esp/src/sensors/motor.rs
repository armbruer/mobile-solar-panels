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
    max_angle: f32,
    step_size: f32,
    current_angle: f32,
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
        max_angle: f32,
        step_size: f32,
    ) -> StepperMotor<OutputPin1, OutputPin2, OutputPin3, OutputPin4> {
        StepperMotor {
            pin1,
            pin2,
            pin3,
            pin4,
            max_angle,
            step_size,
            current_angle: 0.0,
            initalized_angles: false,
        }
    }

    pub fn initAngle(ismax_angle: bool) {
        if ismax_angle {
            self.current_angle = self.max_angle;
        } else {
            self.current_angle = 0.0;
        }
        self.initalized_angles = true;
    }

    pub fn rotatableAngle(&mut self, angle: f32) -> bool {
        if !self.initalized_angles
            || self.current_angle == angle
            || angle < 0.0
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

    pub fn rotatableRight(&mut self) -> bool {
        return self.rotatableAngle(self.max_angle);
    }

    pub fn rotatableLeft(&mut self) -> bool {
        return self.rotatableAngle(0.0);
    }

    pub fn rotateAngleFull(&mut self, motorSpeed: Speed, angle: f32) {
        while !self.rotatableAngle(angle) {
            self.rotateAngle(motorSpeed, angle);
        }
    }

    pub fn rotateAngle(&mut self, motorSpeed: Speed, angle: f32) -> f32 {
        if !self.rotatableAngle(angle) {
            return self.current_angle;
        }
        if self.current_angle < angle {
            return self.rotateRight(motorSpeed);
        } else {
            return self.rotateLeft(motorSpeed);
        }
    }

    pub fn rotateRight(&mut self, motorSpeed: Speed) -> f32 {
        if !self.rotatableRight() {
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
    }

    pub fn rotateLeft(&mut self, motorSpeed: Speed) -> f32 {
        if !self.rotatableLeft() {
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
