use esp_idf_hal::adc::attenuation::DB_2_5;
use esp_idf_hal::adc::attenuation::DB_6;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys as _;
use hd44780_driver::{Display, HD44780};
use std::{thread, time::Duration};
use esp_idf_hal::adc::oneshot::{AdcDriver, AdcChannelDriver};
use esp_idf_sys as _;
use esp_idf_svc::hal::adc::{attenuation::DB_11};

fn main() {
    // ESP-IDF 的必要初始化（載入韌體修補）
    esp_idf_sys::link_patches();

    // 取得 ESP32 所有可用的硬體外設（如 GPIO、I2C 控制器）
    let peripherals = Peripherals::take().unwrap();

    // -----------------------------------
    // 初始化 GPIO2 作為輸出腳位（通常接 LED）
    // -----------------------------------
    let mut led = PinDriver::output(peripherals.pins.gpio2).unwrap();

    // -----------------------------------
    // 初始化 I2C 通訊，用來控制 LCD
    // SDA 接在 GPIO21，SCL 接在 GPIO22
    // -----------------------------------
    let i2c_config = config::Config::default();
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio21, // SDA
        peripherals.pins.gpio22, // SCL
        &i2c_config,
    )
    .unwrap();

    // 建立延遲器（必要給 LCD 控制）
    let mut delay = Ets;

    // -----------------------------------
    // 初始化 LCD (透過 I2C)
    // 裝置位址是 0x27（I2C LCD 常見位址）
    // -----------------------------------
    let mut lcd = HD44780::new_i2c(i2c, 0x27, &mut delay).unwrap();

    // 重設並清除 LCD 畫面
    lcd.reset(&mut delay).unwrap();
    lcd.clear(&mut delay).unwrap();
    lcd.set_display(Display::On, &mut delay).unwrap();

    // 初始畫面顯示
    lcd.clear(&mut delay).unwrap();
    lcd.write_str("Status: HIGH", &mut delay).unwrap();

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

        // LED 打開，LCD 顯示 moisture
        led.set_high().unwrap();
        lcd.clear(&mut delay).unwrap();
        lcd.write_str(
            &format!("{}({})", moisture_level, moisture_value),
            &mut delay
        )
        .unwrap();
        thread::sleep(Duration::from_secs(1));

        // LED 關閉
        led.set_low().unwrap();
        thread::sleep(Duration::from_secs(1));
    }
}
