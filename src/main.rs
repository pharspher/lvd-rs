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

    loop {
        let moisture_value = moisture_sensor.read_value();
        let moisture_level = moisture_sensor.read_level();

        led.on();
        lcd.display(&format!("{}({})", moisture_level, moisture_value));
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

    pub fn display(&mut self, str: &str) {
        self.lcd.clear(&mut self.delay).unwrap();
        self.lcd.write_str(str, &mut self.delay).unwrap();
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
        Self { adc, channel }
    }

    pub fn read_value(&mut self) -> u16 {
        self.adc.read(&mut self.channel).unwrap()
    }

    pub fn read_level(&mut self) -> MoistureLevel {
        let moisture_value = self.read_value();

        let moisture_min = 900;
        let moisture_max = 1412;
        let moisture_step = (moisture_max - moisture_min) / 5;

        match moisture_value {
            v if v <= moisture_min + moisture_step * 1 => MoistureLevel::VeryWet,
            v if v <= moisture_min + moisture_step * 2 => MoistureLevel::Wet,
            v if v <= moisture_min + moisture_step * 3 => MoistureLevel::Normal,
            v if v <= moisture_min + moisture_step * 4 => MoistureLevel::Dry,
            _ => MoistureLevel::VeryDry
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
