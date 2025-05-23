use std::marker::PhantomData;

use esp_idf_hal::{delay::Ets, gpio::IOPin, i2c::{config, I2c, I2cDriver}, peripheral::Peripheral};
use hd44780_driver::{bus::I2CBus, HD44780};

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
        let i2c = I2cDriver::new(i2c, sda, scl, &config::Config::default())
            .expect("Failed to create I2C driver");
        let mut delay = Ets;
        let mut lcd = HD44780::new_i2c(i2c, 0x27, &mut delay)
            .expect("Failed to create LCD controller");

        lcd.reset(&mut delay).expect("Failed to reset LCD");
        lcd.set_cursor_visibility(hd44780_driver::Cursor::Invisible, &mut delay)
            .expect("Failed to set LCD cursor visibility");
        lcd.set_cursor_blink(hd44780_driver::CursorBlink::Off, &mut delay)
            .expect("Failed to set LCD cursor blink");

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
                self.write_char_at(*c, i as u8);
                self.prev_line1[i] = *c;
            }
        }
    }

    pub fn update_second_line(&mut self, line: &str) {
        let new_line = Self::pad_line(line);
        for (i, c) in new_line.iter().enumerate() {
            if self.prev_line2[i] != *c {
                self.write_char_at(*c, 0x40 + i as u8);
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

    fn write_char_at(&mut self, ch: char, pos: u8) {
        self.lcd.set_cursor_pos(pos, &mut self.delay)
            .expect("Failed to set cursor position");
        self.lcd.write_char(ch, &mut self.delay)
            .expect("Failed to write character");
    }
}