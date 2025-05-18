use esp_idf_hal::adc::attenuation::DB_6;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
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
use std::marker::PhantomData;
use std::{thread, time::Duration};
use esp_idf_hal::adc::oneshot::{AdcDriver, AdcChannelDriver};
use esp_idf_sys as _;
use std::fmt;
use chrono::{DateTime, Local, Duration as ChronoDuration};

fn main() {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();

    let mut led = Led::new(peripherals.pins.gpio2);
    let mut lcd = Lcd::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22);

    let adc = AdcDriver::new(peripherals.adc1).unwrap();
    let mut moisture_sensor = MoistureSensor::new(&adc, peripherals.pins.gpio36);

    let mut pump = Pump::new(peripherals.pins.gpio26);

    loop {
        led.on();
        match moisture_sensor.read_avg() {
            Some((m_value, m_level)) => {
                lcd.display_two_lines(&format!("{}({})", m_level, m_value), &pump.elapsed_since_last_on_str());
                if m_level >= MoistureLevel::VeryDry {
                    pump.turn_on();
                    println!("MOTOR ON");
                    lcd.display_second_line(&pump.elapsed_since_last_on_str());
                }
                Ets::delay_ms(1000);
                pump.turn_off();
            },
            None => {

            }
        }

        thread::sleep(Duration::from_secs(1));

        led.off();
        thread::sleep(Duration::from_secs(1));
    }
}

pub struct Led<T: OutputPin> {
    pin_driver: PinDriver<'static, T, Output>
}

impl<T: OutputPin> Led<T> {
    pub fn new(pin: impl Peripheral<P = T> + 'static) -> Self {
        let driver = PinDriver::output(pin).unwrap();

        Self {
            pin_driver: driver
        }
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
    _t: PhantomData<T>,
    _u: PhantomData<U>,
    _v: PhantomData<V>,
}

impl<T: I2c, U: IOPin, V: IOPin> Lcd<T, U, V> {
    pub fn new(
        i2c: impl Peripheral<P = T> + 'static,
        sda: impl Peripheral<P = U> + 'static,
        scl: impl Peripheral<P = V> + 'static
    ) -> Self {
        let i2c = I2cDriver::new(i2c, sda, scl, &config::Config::default()).unwrap();

        let mut ets = Ets;
        let mut lcd = HD44780::new_i2c(i2c, 0x27, &mut ets).unwrap();
        lcd.reset(&mut ets).unwrap();

        Self {
            lcd: lcd,
            delay: ets,
            _t: PhantomData,
            _u: PhantomData,
            _v: PhantomData
        }
    }

    pub fn display_first_line(&mut self, line1: &str) {
        self.lcd.set_cursor_pos(0x00, &mut self.delay).unwrap();
        let mut s = line1.chars().take(16).collect::<String>();
        while s.len() < 16 {
            s.push(' ');
        }
        self.lcd.write_str(&s, &mut self.delay).unwrap();
    }

    pub fn display_second_line(&mut self, line2: &str) {
        self.lcd.set_cursor_pos(0x40, &mut self.delay).unwrap();
        let mut s = line2.chars().take(16).collect::<String>();
        while s.len() < 16 {
            s.push(' ');
        }
        self.lcd.write_str(&s, &mut self.delay).unwrap();
    }

    pub fn display_two_lines(&mut self, line1: &str, line2: &str) {
        self.lcd.clear(&mut self.delay).unwrap();
        self.lcd.set_cursor_pos(0, &mut self.delay).unwrap();
        self.lcd.write_str(line1, &mut self.delay).unwrap();

        self.lcd.set_cursor_pos(0x40, &mut self.delay).unwrap();
        self.lcd.write_str(line2, &mut self.delay).unwrap();
    }
}

pub struct MoistureSensor<'a, A, P>
where
    A: Adc + 'a,
    P: ADCPin + 'a,
    &'a AdcDriver<'a, A>: Borrow<AdcDriver<'a, <P as ADCPin>::Adc>>
{
    adc: &'a AdcDriver<'a, A>,
    channel: AdcChannelDriver<'a, P, &'a AdcDriver<'a, A>>,
    history: [u16; 3],
    curr_pos: usize
}

impl<'a, A, P> MoistureSensor<'a, A, P>
where
    A: Adc + 'a,
    P: ADCPin + 'a,
    &'a AdcDriver<'a, A>: Borrow<AdcDriver<'a, <P as ADCPin>::Adc>>
{
    pub fn new(adc: &'a AdcDriver<'a, A>, pin: P) -> Self {
        let config = AdcChannelConfig {
            attenuation: DB_6,
            ..Default::default()
        };
        let channel = AdcChannelDriver::new(adc, pin, &config).unwrap();
        Self { adc, channel, history: [0; 3], curr_pos: 0 }
    }

    pub fn read(&mut self) -> (u16, MoistureLevel) {
        let value = self.adc.read(&mut self.channel).unwrap();
        (value, self.to_moisture_level(value))
    }

    pub fn read_avg(&mut self) -> Option<(u16, MoistureLevel)> {
        let value = self.adc.read(&mut self.channel).unwrap();
        self.history[self.curr_pos] = value;
        self.curr_pos = (self.curr_pos + 1) % self.history.len();

        if self.history.iter().any(|&v| v == 0) {
            return None;
        }
        
        let sum: u32 = self.history.iter().map(|&v| v as u32).sum();
        let avg = (sum / self.history.len() as u32) as u16;        

        Some((avg, self.to_moisture_level(avg)))
    }

    fn to_moisture_level(&self, value: u16) -> MoistureLevel {
        let moisture_min = 900;
        let moisture_max = 1412;
        let moisture_step = (moisture_max - moisture_min) / 5;

        match value {
            v if v <= moisture_min + moisture_step * 1 => MoistureLevel::VeryWet,
            v if v <= moisture_min + moisture_step * 2 => MoistureLevel::Wet,
            v if v <= moisture_min + moisture_step * 3 => MoistureLevel::Normal,
            v if v <= moisture_min + moisture_step * 4 => MoistureLevel::Dry,
            _ => MoistureLevel::VeryDry
        }
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
            MoistureLevel::VeryWet => "Very wet",
            MoistureLevel::Wet => "Wet",
            MoistureLevel::Normal => "Normal",
            MoistureLevel::Dry => "Dry",
            MoistureLevel::VeryDry => "Very dry",
        };
        write!(f, "{}", label)
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

    pub fn turn_on(&mut self) {
        self.pin.set_high().unwrap();
        self.last_on = Some(Local::now());
    }

    pub fn turn_off(&mut self) {
        self.pin.set_low().unwrap();
    }

    pub fn time_since_last_on(&self) -> Option<ChronoDuration> {
        self.last_on.map(|t| Local::now() - t)
    }

    pub fn last_on_time_str(&self) -> String {
        match self.last_on {
            Some(dt) => dt.format("%H:%M:%S").to_string(),
            None => "Never".to_string(),
        }
    }

    pub fn elapsed_since_last_on_str(&self) -> String {
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
}
