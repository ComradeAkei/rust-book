//! Раздел 27.5: драйвер сдвигового регистра 74HC595 — восемь выходов по
//! трём проводам. Драйвер знает только трейт `OutputPin` из embedded-hal
//! 1.0: на хосте его проверяют модели выводов, на LM3S6965 он двигает
//! выводы порта через HAL code/ch27-hal.

use embedded_hal::digital::{OutputPin, PinState};

// ANCHOR: driver
/// 74HC595: по фронту SRCLK (`clock`) бит с SER (`data`) вдвигается в
/// сдвиговый регистр, по фронту RCLK (`latch`) регистр защёлкивается на
/// выходы Q0–Q7.
pub struct Hc595<D, C, L> {
    data: D,
    clock: C,
    latch: L,
}

impl<D, C, L, E> Hc595<D, C, L>
where
    D: OutputPin<Error = E>,
    C: OutputPin<Error = E>,
    L: OutputPin<Error = E>,
{
    pub fn new(data: D, clock: C, latch: L) -> Self {
        Hc595 { data, clock, latch }
    }

    /// Выдвинуть байт старшим битом вперёд и защёлкнуть: бит 7 окажется
    /// на Q7, бит 0 — на Q0.
    pub fn write(&mut self, byte: u8) -> Result<(), E> {
        for i in (0..8).rev() {
            self.data.set_state(PinState::from(byte >> i & 1 != 0))?;
            self.clock.set_high()?;
            self.clock.set_low()?;
        }
        self.latch.set_high()?;
        self.latch.set_low()
    }

    /// Вернуть выводы: драйвер больше не нужен, выводы — нужны.
    pub fn release(self) -> (D, C, L) {
        (self.data, self.clock, self.latch)
    }
}
// ANCHOR_END: driver

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::vec::Vec;

    // ANCHOR: model
    /// Модель микросхемы: что она видит на трёх входах.
    #[derive(Default)]
    struct Chip {
        ser: bool,
        srclk: bool,
        rclk: bool,
        shift: u8,
        out: u8,
        edges: Vec<char>,
    }

    impl Chip {
        fn pin(&mut self, which: char, level: bool) {
            match which {
                'D' => self.ser = level,
                'C' => {
                    if level && !self.srclk {
                        self.shift = self.shift << 1 | self.ser as u8;
                        self.edges.push('C');
                    }
                    self.srclk = level;
                }
                'L' => {
                    if level && !self.rclk {
                        self.out = self.shift;
                        self.edges.push('L');
                    }
                    self.rclk = level;
                }
                _ => unreachable!(),
            }
        }
    }

    /// Вывод, подключённый к входу `which` модели.
    struct Wire(Rc<RefCell<Chip>>, char);

    impl embedded_hal::digital::ErrorType for Wire {
        type Error = Infallible;
    }

    impl OutputPin for Wire {
        fn set_low(&mut self) -> Result<(), Infallible> {
            self.0.borrow_mut().pin(self.1, false);
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Infallible> {
            self.0.borrow_mut().pin(self.1, true);
            Ok(())
        }
    }
    // ANCHOR_END: model

    #[test]
    fn byte_reaches_outputs() {
        let chip = Rc::new(RefCell::new(Chip::default()));
        let w = |c| Wire(Rc::clone(&chip), c);
        let mut sr = Hc595::new(w('D'), w('C'), w('L'));
        for byte in [0xA5, 0x00, 0xFF, 0x01, 0x80] {
            sr.write(byte).unwrap();
            assert_eq!(chip.borrow().out, byte);
        }
        // Восемь фронтов SRCLK и один RCLK на байт.
        let edges: std::string::String = chip.borrow().edges.iter().take(9).collect();
        assert_eq!(edges, "CCCCCCCCL");
        assert_eq!(chip.borrow().edges.len(), 5 * 9);
    }

    /// Вывод, который отказывает после `ok` переключений.
    struct Flaky(u32);

    #[derive(Debug, PartialEq)]
    struct Broken;

    impl embedded_hal::digital::Error for Broken {
        fn kind(&self) -> embedded_hal::digital::ErrorKind {
            embedded_hal::digital::ErrorKind::Other
        }
    }

    impl embedded_hal::digital::ErrorType for Flaky {
        type Error = Broken;
    }

    impl OutputPin for Flaky {
        fn set_low(&mut self) -> Result<(), Broken> {
            self.set_high()
        }
        fn set_high(&mut self) -> Result<(), Broken> {
            if self.0 == 0 {
                return Err(Broken);
            }
            self.0 -= 1;
            Ok(())
        }
    }

    #[test]
    fn error_stops_the_transfer() {
        // Тактовый вывод отказывает на третьем фронте: ошибка выходит из
        // write, защёлка не дёргается.
        let mut sr = Hc595::new(Flaky(100), Flaky(2), Flaky(100));
        assert_eq!(sr.write(0xFF), Err(Broken));
        let (data, clock, latch) = sr.release();
        assert_eq!((data.0, clock.0, latch.0), (98, 0, 100));
    }
}
