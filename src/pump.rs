use std::time::{Duration, Instant};

use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;

pub struct Pump<T: OutputPin> {
    pin: PinDriver<'static, T, Output>,
    last_on: Option<Instant>,
}

impl<T: OutputPin> Pump<T> {
    pub fn new(pin: impl Peripheral<P = T> + 'static) -> Self {
        let mut driver = PinDriver::output(pin).expect("Failed to create pin driver");
        driver.set_low().expect("Failed to turn off pump");

        Self {
            pin: driver,
            last_on: None,
        }
    }

    pub fn on(&mut self) {
        self.pin.set_high().expect("Failed to turn on pump");
        self.last_on = Some(Instant::now());
    }

    pub fn off(&mut self) {
        self.pin.set_low().expect("Failed to turn off pump");
    }

    pub fn time_since_last_on_str(&self) -> String {
        match self.time_since_last_on() {
            Some(duration) => {
                let secs = duration.as_secs();
                if secs < 60 {
                    format!("{} sec ago", secs)
                } else if secs < 3600 {
                    format!("{} min ago", secs / 60)
                } else {
                    format!("{} hr ago", secs / 3600)
                }
            }
            None => "Never".to_string(),
        }
    }

    fn time_since_last_on(&self) -> Option<Duration> {
        self.last_on.map(|t| Instant::now().duration_since(t))
    }
}
