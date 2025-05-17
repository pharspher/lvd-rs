use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys as _;
use hd44780_driver::{Display, HD44780};
use std::{thread, time::Duration};

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
    // 主迴圈：每秒切換 LED 與 LCD 顯示內容
    // -----------------------------------
    loop {
        // LED 打開，LCD 顯示 Daisy
        led.set_high().unwrap();
        println!("HIGH");
        lcd.clear(&mut delay).unwrap();
        lcd.write_str("Daisy", &mut delay).unwrap();
        thread::sleep(Duration::from_secs(1));

        // LED 關閉，LCD 顯示 Paxon
        led.set_low().unwrap();
        println!("LOW");
        lcd.clear(&mut delay).unwrap();
        lcd.write_str("Paxon", &mut delay).unwrap();
        thread::sleep(Duration::from_secs(1));
    }
}
