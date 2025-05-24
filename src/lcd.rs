use std::array;
use std::marker::PhantomData;

use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::IOPin;
use esp_idf_hal::i2c::{config, I2c, I2cDriver};
use esp_idf_hal::peripheral::Peripheral;
use hd44780_driver::bus::I2CBus;
use hd44780_driver::HD44780;

pub struct Lcd<T, U, V>
where
    T: I2c,
    U: IOPin,
    V: IOPin,
{
    controller: HD44780<I2CBus<I2cDriver<'static>>>,
    line1: LcdLine,
    line2: LcdLine,
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
        let mut controller =
            HD44780::new_i2c(i2c, 0x27, &mut delay).expect("Failed to create LCD controller");

        controller.reset(&mut delay).expect("Failed to reset LCD");
        controller
            .set_cursor_visibility(hd44780_driver::Cursor::Invisible, &mut delay)
            .expect("Failed to set LCD cursor visibility");
        controller
            .set_cursor_blink(hd44780_driver::CursorBlink::Off, &mut delay)
            .expect("Failed to set LCD cursor blink");

        Self {
            controller,
            line1: LcdLine::new(0x00),
            line2: LcdLine::new(0x40),
            _t: PhantomData,
            _u: PhantomData,
            _v: PhantomData,
        }
    }

    pub fn update_first_line(&mut self, line: &str) {
        self.line1.update(line, &mut self.controller);
    }

    pub fn update_second_line(&mut self, line: &str) {
        self.line2.update(line, &mut self.controller);
    }

    pub fn update_two_lines(&mut self, line1: &str, line2: &str) {
        self.update_first_line(line1);
        self.update_second_line(line2);
    }
}

struct LcdLine {
    cells: [LcdChar; 16],
}

impl LcdLine {
    fn new(line_address: u8) -> Self {
        Self {
            cells: array::from_fn(|i| LcdChar {
                last_ch: ' ',
                pos: line_address + i as u8,
            }),
        }
    }

    fn update(&mut self, line: &str, writer: &mut impl LcdCharWriter) {
        line.chars()
            .chain([' '; 16])
            .take(16)
            .zip(self.cells.iter_mut())
            .for_each(|(ch, cell)| {
                cell.write(&ch, writer);
            });
    }
}

struct LcdChar {
    last_ch: char,
    pos: u8,
}

impl LcdChar {
    fn write(&mut self, ch: &char, writer: &mut impl LcdCharWriter) {
        if self.last_ch != *ch {
            writer.write(*ch, self.pos);
            self.last_ch = *ch;
        }
    }
}

trait LcdCharWriter {
    fn write(&mut self, ch: char, pos: u8);
}

impl LcdCharWriter for HD44780<I2CBus<I2cDriver<'static>>> {
    fn write(&mut self, ch: char, pos: u8) {
        self.set_cursor_pos(pos, &mut Ets)
            .expect("Failed to set cursor position");
        self.write_char(ch, &mut Ets)
            .expect("Failed to write character");
    }
}
