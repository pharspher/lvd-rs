use chrono::{DateTime, Duration as ChronoDuration, Local};
use esp_idf_hal::adc::attenuation::DB_6;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::adc::Adc;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys as _;
use hd44780_driver::bus::I2CBus;
use hd44780_driver::HD44780;
use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;
use std::{thread, time::Duration};

fn main() {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();

    let mut led = Led::new(peripherals.pins.gpio2);
    let mut lcd = Lcd::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
    );
    let mut pump = Pump::new(peripherals.pins.gpio26);

    let adc = AdcDriver::new(peripherals.adc1).unwrap();
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
        let driver = PinDriver::output(pin).unwrap();

        Self { pin_driver: driver }
    }

    pub fn on(&mut self) {
        self.pin_driver.set_high().unwrap();
    }

    pub fn off(&mut self) {
        self.pin_driver.set_low().unwrap();
    }
}

pub struct Lcd<T: I2c, U: IOPin, V: IOPin> {
    lcd: HD44780<I2CBus<I2cDriver<'static>>>,
    delay: Ets,
    prev_line1: [char; 16],
    prev_line2: [char; 16],
    _t: PhantomData<T>,
    _u: PhantomData<U>,
    _v: PhantomData<V>,
}

impl<T: I2c, U: IOPin, V: IOPin> Lcd<T, U, V> {
    pub fn new(
        i2c: impl Peripheral<P = T> + 'static,
        sda: impl Peripheral<P = U> + 'static,
        scl: impl Peripheral<P = V> + 'static,
    ) -> Self {
        let i2c = I2cDriver::new(i2c, sda, scl, &config::Config::default()).unwrap();
        let mut delay = Ets;
        let mut lcd = HD44780::new_i2c(i2c, 0x27, &mut delay).unwrap();
        lcd.reset(&mut delay).unwrap();
        lcd.set_cursor_visibility(hd44780_driver::Cursor::Invisible, &mut delay)
            .unwrap();
        lcd.set_cursor_blink(hd44780_driver::CursorBlink::Off, &mut delay)
            .unwrap();

        Self {
            lcd,
            delay,
            prev_line1: [' '; 16],
            prev_line2: [' '; 16],
            _t: PhantomData,
            _u: PhantomData,
            _v: PhantomData,
        }
    }

    pub fn update_first_line(&mut self, line: &str) {
        let new_line = Self::pad_line(line);
        for (i, c) in new_line.iter().enumerate() {
            if self.prev_line1[i] != *c {
                self.lcd.set_cursor_pos(i as u8, &mut self.delay).unwrap();
                self.lcd.write_char(*c, &mut self.delay).unwrap();
                self.prev_line1[i] = *c;
            }
        }
    }

    pub fn update_second_line(&mut self, line: &str) {
        let new_line = Self::pad_line(line);
        for (i, c) in new_line.iter().enumerate() {
            if self.prev_line2[i] != *c {
                self.lcd
                    .set_cursor_pos(0x40 + i as u8, &mut self.delay)
                    .unwrap();
                self.lcd.write_char(*c, &mut self.delay).unwrap();
                self.prev_line2[i] = *c;
            }
        }
    }

    pub fn update_two_lines(&mut self, line1: &str, line2: &str) {
        self.update_first_line(line1);
        self.update_second_line(line2);
    }

    fn pad_line(s: &str) -> [char; 16] {
        let mut result = [' '; 16];
        for (i, c) in s.chars().take(16).enumerate() {
            result[i] = c;
        }
        result
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
        let channel = AdcChannelDriver::new(adc, pin, &config).unwrap();
        Self {
            adc,
            channel,
            recent_samples: [0; 3],
            sample_idx: 0,
        }
    }

    pub fn read(&mut self) -> (u16, MoistureLevel) {
        let value = self.adc.read(&mut self.channel).unwrap();
        (value, MoistureLevel::from_value(value))
    }

    pub fn read_avg(&mut self) -> Option<(u16, MoistureLevel)> {
        let value = self.adc.read(&mut self.channel).unwrap();
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
    last_on: Option<DateTime<Local>>,
}

impl<T: OutputPin> Pump<T> {
    pub fn new(pin: impl Peripheral<P = T> + 'static) -> Self {
        let mut driver = PinDriver::output(pin).unwrap();
        driver.set_low().unwrap();

        Self {
            pin: driver,
            last_on: None,
        }
    }

    pub fn on(&mut self) {
        self.pin.set_high().unwrap();
        self.last_on = Some(Local::now());
    }

    pub fn off(&mut self) {
        self.pin.set_low().unwrap();
    }

    pub fn time_since_last_on_str(&self) -> String {
        match self.time_since_last_on() {
            Some(duration) => {
                let secs = duration.num_seconds();
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

    fn time_since_last_on(&self) -> Option<ChronoDuration> {
        self.last_on.map(|t| Local::now() - t)
    }
}
