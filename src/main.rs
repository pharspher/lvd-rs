use esp_idf_hal::gpio::*;
use esp_idf_hal::i2c::*;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys as _;
use hd44780_driver::{HD44780, Display};
use std::{thread, time::Duration};

fn main() {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();

    let mut led = PinDriver::output(peripherals.pins.gpio2).unwrap();

    let i2c_config = config::Config::default();
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &i2c_config,
    )
    .unwrap();

    let mut delay = Ets;

    let mut lcd = HD44780::new_i2c(i2c, 0x27, &mut delay).unwrap();
    lcd.reset(&mut delay).unwrap();
    lcd.clear(&mut delay).unwrap();
    lcd.set_display(Display::On, &mut delay).unwrap();

    lcd.clear(&mut delay).unwrap();
    lcd.write_str("Status: HIGH", &mut delay).unwrap();

    loop {
        led.set_high().unwrap();
        println!("HIGH");
         lcd.clear(&mut delay).unwrap();
         lcd.write_str("Daisy", &mut delay).unwrap();
        thread::sleep(Duration::from_secs(1));

        led.set_low().unwrap();
        println!("LOW");
         lcd.clear(&mut delay).unwrap();
         lcd.write_str("Paxon", &mut delay).unwrap();
        thread::sleep(Duration::from_secs(1));
    }
}
