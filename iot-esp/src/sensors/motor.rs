use esp_idf_hal::{
    gpio::{OutputPin},
};
use esp_idf_sys::EspError;

use std::thread;
use std::time::Duration;

use embedded_hal::digital::blocking::OutputPin;

use esp_idf_hal::peripherals::Peripherals;

pub struct StepperMotor {
    in1: OutputPin,
    in2: OutputPin,
    in3: OutputPin,
    in4: OutputPin,
}

impl StepperMotor {
    pub fn new(
        pin1: OutputPin,
        pin2: OutputPin,
        pin3: OutputPin,
        pin4: OutputPin,
    ) -> Result<StepperMotor, EspError> {

        let in1 = pin1.into_output()?
        let in2 = pin2.into_output()?
        let in3 = pin3.into_output()?
        let in4 = pin4.into_output()?

        Ok(StepperMotor { in1,  in2, in3, in4})
    }

    pub fn rotateRight(motorSpeed: i32)
    {
        setMotor(false, false, false, true, motorSpeed);
        setMotor(false, false, true, true, motorSpeed);
        setMotor(false, false, true, false, motorSpeed);
        setMotor(false, true, true, false, motorSpeed);
        setMotor(false, true, false, false, motorSpeed);
        setMotor(true, true, false, false, motorSpeed);
        setMotor(true, false, false, false, motorSpeed);
        setMotor(true, false, false, true, motorSpeed);
    }

    pub fn rotateLeft(motorSpeed: i32)
    {
        setMotor(true, false, false, false, motorSpeed);
        setMotor(true, true, false, false, motorSpeed);
        setMotor(false, true, false, false, motorSpeed);
        setMotor(false, true, true, false, motorSpeed);
        setMotor(false, false, true, false, motorSpeed);
        setMotor(false, false, true, true, motorSpeed);
        setMotor(false, false, false, true, motorSpeed);
        setMotor(true, false, false, true, motorSpeed);
    }

    pub fn stopMotor()
    {
        setMotor(false, false, false, false, 0);
    }

    pub fn setMotor(&mut self, in1: bool, in2: bool, in3: bool, in4: bool, motorSpeed: i32)
    {
        setPin(self.in1, in1);
        setPin(self.in2, in2);
        setPin(self.in3, in3);
        setPin(self.in4, in4);
        thread::sleep(Duration::from_millis(motorSpeed));
    }

    pub fn setPin(pin: OutputPin, high: bool)
    {
        if high {
            pin.set_high()?
        }
        else {
            pin.set_low()?
        }
    }
}
