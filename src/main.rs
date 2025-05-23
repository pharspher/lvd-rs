mod led;
use led::Led;

mod lcd;
use lcd::Lcd;

mod moister;
use moister::MoistureLevel;
use moister::MoistureSensor;

mod pump;
use pump::Pump;

use esp_idf_hal::adc::oneshot::AdcDriver;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::peripherals::Peripherals;
use std::thread;
use std::time::Duration;

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
