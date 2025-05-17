use esp_idf_hal::adc::attenuation::DB_6;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys as _;
use hd44780_driver::bus::I2CBus;
use hd44780_driver::HD44780;
use std::marker::PhantomData;
use std::{thread, time::Duration};
use esp_idf_hal::adc::oneshot::{AdcDriver, AdcChannelDriver};
use esp_idf_sys as _;

fn main() {
    // ESP-IDF 的必要初始化（載入韌體修補）
    esp_idf_sys::link_patches();

    // 取得 ESP32 所有可用的硬體外設（如 GPIO、I2C 控制器）
    let peripherals = Peripherals::take().unwrap();

    let mut led = Led::new(peripherals.pins.gpio2);
    let mut lcd = Lcd::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22);

    // -----------------------------------
    // 初始化土壤濕度偵測器
    // 接在 GPIO36
    // -----------------------------------
    let adc = AdcDriver::new(peripherals.adc1).unwrap();

     // configuring pin to analog read, you can regulate the adc input voltage range depending on your need
     // for this example we use the attenuation of 11db which sets the input voltage range to around 0-3.6V
     let config = AdcChannelConfig {
         attenuation: DB_6,
         ..Default::default()
     };
     let mut adc_pin = AdcChannelDriver::new(
        &adc,
        peripherals.pins.gpio36,
        &config
    )
    .unwrap();

    // -----------------------------------
    // 主迴圈：每秒切換 LED 與 LCD 顯示內容
    // -----------------------------------
    loop {
        let moisture_value = adc.read(&mut adc_pin).unwrap();
        let moisture_min = 900;
        let moisture_max = 1412;
        let moisture_step = (moisture_max - moisture_min) / 5;
        let moisture_level = match moisture_value {
            v if v <= moisture_min + moisture_step * 1 => "Very wet",  // Level 1
            v if v <= moisture_min + moisture_step * 2 => "Wet",       // Level 2
            v if v <= moisture_min + moisture_step * 3 => "Normal",    // Level 3
            v if v <= moisture_min + moisture_step * 4 => "Dry",       // Level 4
            _ => "Very dry",                                                // Level 5
        };

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
