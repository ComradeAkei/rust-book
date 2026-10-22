//! Задача D: драйвер датчика температуры TMP-9 поверх embedded-hal 1.0,
//! один и тот же для I2C и SPI.

use embedded_hal::i2c::I2c;
use embedded_hal::spi::{Operation, SpiDevice};

/// Регистр «кто я»: у TMP-9 в нём [`ID`].
pub const WHO_AM_I: u8 = 0x0F;
/// Регистр управления.
pub const CTRL: u8 = 0x20;
/// Регистр состояния.
pub const STATUS: u8 = 0x27;
/// Младший байт температуры; старший — следующий регистр.
pub const TEMP_L: u8 = 0x2A;
/// Ответ TMP-9 на чтение `WHO_AM_I`.
pub const ID: u8 = 0xA9;
/// CTRL: непрерывное измерение.
pub const CTRL_RUN: u8 = 0x01;
/// STATUS: готово новое измерение.
pub const STATUS_DRDY: u8 = 0x01;
/// Сколько раз опросить STATUS, прежде чем сдаться.
pub const POLLS: usize = 10;

/// Ошибка драйвера. `E` — ошибка шины, у I2C и SPI она своя.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<E> {
    /// Шина не справилась: нет ответа, потеря арбитража и т. п.
    Bus(E),
    /// На шине не TMP-9: `WHO_AM_I` вернул другое значение.
    WrongId(u8),
    /// Измерение не готово за [`POLLS`] опросов.
    Timeout,
}

/// Регистры датчика, доступные через какую-то шину. Добавьте методы
/// чтения и записи регистров и реализуйте их для обеих шин.
pub trait Regs {
    /// Ошибка этой шины.
    type Error;
}

/// TMP-9 на шине I2C по адресу `addr`.
pub struct I2cRegs<I> {
    i2c: I,
    addr: u8,
}

/// TMP-9 на шине SPI; выбор кристалла — забота `SpiDevice`.
pub struct SpiRegs<S> {
    spi: S,
}

impl<I: I2c> Regs for I2cRegs<I> {
    type Error = I::Error;
}

impl<S: SpiDevice> Regs for SpiRegs<S> {
    type Error = S::Error;
}

/// Драйвер TMP-9. Логика одна, шина — параметр типа.
pub struct Tmp9<R> {
    regs: R,
}

impl<R: Regs> Tmp9<R> {
    /// Температура в сотых долях градуса Цельсия.
    pub fn temperature(&mut self) -> Result<i16, Error<R::Error>> {
        todo!("опросить STATUS не больше POLLS раз, затем прочитать TEMP")
    }
}

impl<I: I2c> Tmp9<I2cRegs<I>> {
    /// Проверить, что на шине TMP-9, и запустить измерения.
    pub fn new_i2c(i2c: I, addr: u8) -> Result<Self, Error<I::Error>> {
        todo!()
    }

    /// Вернуть шину.
    pub fn release(self) -> I {
        todo!()
    }
}

impl<S: SpiDevice> Tmp9<SpiRegs<S>> {
    /// Проверить, что на шине TMP-9, и запустить измерения.
    pub fn new_spi(spi: S) -> Result<Self, Error<S::Error>> {
        todo!()
    }

    /// Вернуть шину.
    pub fn release(self) -> S {
        todo!()
    }
}
