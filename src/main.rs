mod lcd;
use lcd::Lcd;

use esp_idf_hal::adc::attenuation::DB_6;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::adc::Adc;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{ADCPin, Output, OutputPin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use std::borrow::Borrow;
use std::fmt;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take()
        .expect("Failed to acquire peripherals");

    let mut led = Led::new(peripherals.pins.gpio2);
    let mut lcd = Lcd::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
    );
    let mut pump = Pump::new(peripherals.pins.gpio26);

    let adc = AdcDriver::new(peripherals.adc1)
        .expect("Failed to create ADC driver");
    let mut moisture = MoistureSensor::new(&adc, peripherals.pins.gpio36);

    loop {
        led.on();

        if let Some((m_value, m_level)) = moisture.read_avg() {
            lcd.update_two_lines(
                &format!("{}({})", m_level, m_value),
                &pump.time_since_last_on_str(),
            );

            if m_level >= MoistureLevel::VeryDry {
                pump.on();
                lcd.update_second_line(&pump.time_since_last_on_str());
            }

            Ets::delay_ms(1000);
            pump.off();
        }

        thread::sleep(Duration::from_secs(1));

        led.off();
        thread::sleep(Duration::from_secs(1));
    }
}

pub struct Led<T: OutputPin> {
    pin_driver: PinDriver<'static, T, Output>,
}

impl<T: OutputPin> Led<T> {
    pub fn new(pin: impl Peripheral<P = T> + 'static) -> Self {
        let driver = PinDriver::output(pin)
            .expect("Failed to create pin driver");

        Self { pin_driver: driver }
    }

    pub fn on(&mut self) {
        self.pin_driver.set_high().expect("Failed to turn on LED");
    }

    pub fn off(&mut self) {
        self.pin_driver.set_low().expect("Failed to turn off LED");
    }
}

pub struct MoistureSensor<'a, A, P>
where
    A: Adc + 'a,
    P: ADCPin + 'a,
    &'a AdcDriver<'a, A>: Borrow<AdcDriver<'a, <P as ADCPin>::Adc>>,
{
    adc: &'a AdcDriver<'a, A>,
    channel: AdcChannelDriver<'a, P, &'a AdcDriver<'a, A>>,
    recent_samples: [u16; 3],
    sample_idx: usize,
}

impl<'a, A, P> MoistureSensor<'a, A, P>
where
    A: Adc + 'a,
    P: ADCPin + 'a,
    &'a AdcDriver<'a, A>: Borrow<AdcDriver<'a, <P as ADCPin>::Adc>>,
{
    pub fn new(adc: &'a AdcDriver<'a, A>, pin: P) -> Self {
        let config = AdcChannelConfig {
            attenuation: DB_6,
            ..Default::default()
        };
        let channel = AdcChannelDriver::new(adc, pin, &config)
            .expect("Failed to create ADC channel driver");
        Self {
            adc,
            channel,
            recent_samples: [0; 3],
            sample_idx: 0,
        }
    }

    pub fn read(&mut self) -> (u16, MoistureLevel) {
        let value = self.adc.read(&mut self.channel).expect("Failed to read ADC");
        (value, MoistureLevel::from_value(value))
    }

    pub fn read_avg(&mut self) -> Option<(u16, MoistureLevel)> {
        let value = self.adc.read(&mut self.channel).expect("Failed to read ADC");
        self.recent_samples[self.sample_idx] = value;
        self.sample_idx = (self.sample_idx + 1) % self.recent_samples.len();

        if self.recent_samples.iter().any(|&v| v == 0) {
            return None;
        }

        let sum: u32 = self.recent_samples.iter().map(|&v| v as u32).sum();
        let avg = (sum / self.recent_samples.len() as u32) as u16;

        Some((avg, MoistureLevel::from_value(avg)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MoistureLevel {
    VeryWet,
    Wet,
    Normal,
    Dry,
    VeryDry,
}

impl fmt::Display for MoistureLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            MoistureLevel::VeryWet => "Super wet",
            MoistureLevel::Wet => "Quite wet",
            MoistureLevel::Normal => "Normal   ",
            MoistureLevel::Dry => "Quite dry",
            MoistureLevel::VeryDry => "Super dry",
        };
        write!(f, "{}", label)
    }
}

impl MoistureLevel {
    pub fn from_value(value: u16) -> Self {
        let moisture_min = 900;
        let moisture_max = 1412;
        let moisture_step = (moisture_max - moisture_min) / 5;

        match value {
            v if v <= moisture_min + moisture_step => MoistureLevel::VeryWet,
            v if v <= moisture_min + moisture_step * 2 => MoistureLevel::Wet,
            v if v <= moisture_min + moisture_step * 3 => MoistureLevel::Normal,
            v if v <= moisture_min + moisture_step * 4 => MoistureLevel::Dry,
            _ => MoistureLevel::VeryDry,
        }
    }
}

pub struct Pump<T: OutputPin> {
    pin: PinDriver<'static, T, Output>,
    last_on: Option<Instant>,
}

impl<T: OutputPin> Pump<T> {
    pub fn new(pin: impl Peripheral<P = T> + 'static) -> Self {
        let mut driver = PinDriver::output(pin)
            .expect("Failed to create pin driver");
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
