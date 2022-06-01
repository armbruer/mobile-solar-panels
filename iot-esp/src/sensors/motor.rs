use embedded_hal::digital::v2::PinState;
//use esp_idf_hal::gpio::OutputPin;
use esp_idf_sys::EspError;

use std::thread;
use std::time::Duration;

pub struct StepperMotor<
    OUTPUT_PIN1: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
    OUTPUT_PIN2: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
    OUTPUT_PIN3: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
    OUTPUT_PIN4: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
> {
    pin1: OUTPUT_PIN1,
    pin2: OUTPUT_PIN2,
    pin3: OUTPUT_PIN3,
    pin4: OUTPUT_PIN4,
}

impl<
        OUTPUT_PIN1: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
        OUTPUT_PIN2: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
        OUTPUT_PIN3: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
        OUTPUT_PIN4: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
    > StepperMotor<OUTPUT_PIN1, OUTPUT_PIN2, OUTPUT_PIN3, OUTPUT_PIN4>
{
    pub fn new(
        pin1: OUTPUT_PIN1,
        pin2: OUTPUT_PIN2,
        pin3: OUTPUT_PIN3,
        pin4: OUTPUT_PIN4,
    ) -> Result<StepperMotor<OUTPUT_PIN1, OUTPUT_PIN2, OUTPUT_PIN3, OUTPUT_PIN4>, EspError> {
        Ok(StepperMotor {
            pin1,
            pin2,
            pin3,
            pin4,
        })
    }

    pub fn rotateRight(&mut self, motorSpeed: u32) {
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

    pub fn rotateLeft(&mut self, motorSpeed: u32) {
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
            0,
        );
    }

    pub fn setMotor(
        &mut self,
        in1: PinState,
        in2: PinState,
        in3: PinState,
        in4: PinState,
        motorSpeed: u32,
    ) {
        self.pin1.set_state(in1);
        self.pin2.set_state(in2);
        self.pin3.set_state(in3);
        self.pin4.set_state(in4);
        thread::sleep(Duration::from_micros(motorSpeed as u64));
    }
}
