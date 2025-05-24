use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;

pub struct Led<T: OutputPin> {
    pin_driver: PinDriver<'static, T, Output>,
}

impl<T: OutputPin> Led<T> {
    pub fn new(pin: impl Peripheral<P = T> + 'static) -> Self {
        let driver = PinDriver::output(pin).expect("Failed to create pin driver");

        Self { pin_driver: driver }
    }

    pub fn on(&mut self) {
        self.pin_driver.set_high().expect("Failed to turn on LED");
    }

    pub fn off(&mut self) {
        self.pin_driver.set_low().expect("Failed to turn off LED");
    }
}
