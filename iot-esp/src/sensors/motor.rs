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
    ) -> StepperMotor<OutputPin1, OutputPin2, OutputPin3, OutputPin4> {
        StepperMotor {
            pin1,
            pin2,
            pin3,
            pin4,
        }
    }

    pub fn rotateRight(&mut self, motorSpeed: Speed) {
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
    }

    pub fn rotateLeft(&mut self, motorSpeed: Speed) {
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
