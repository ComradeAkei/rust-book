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

// ANCHOR: from
/// Чтобы `?` превращал ошибку шины в ошибку драйвера.
impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Bus(e)
    }
}
// ANCHOR_END: from

// ANCHOR: regs
/// Регистры датчика, доступные через какую-то шину.
pub trait Regs {
    /// Ошибка этой шины.
    type Error;

    /// Прочитать `buf.len()` регистров подряд, начиная с `reg`, одной
    /// транзакцией.
    fn read(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Записать один регистр.
    fn write(&mut self, reg: u8, value: u8) -> Result<(), Self::Error>;
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

    fn read(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), I::Error> {
        // Запись номера регистра и чтение — одна транзакция с повторным
        // START между ними: другой ведущий не вклинится.
        self.i2c.write_read(self.addr, &[reg], buf)
    }

    fn write(&mut self, reg: u8, value: u8) -> Result<(), I::Error> {
        self.i2c.write(self.addr, &[reg, value])
    }
}

impl<S: SpiDevice> Regs for SpiRegs<S> {
    type Error = S::Error;

    fn read(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), S::Error> {
        // Старший бит первого байта — чтение. CS держится всю транзакцию.
        self.spi.transaction(&mut [Operation::Write(&[0x80 | reg]), Operation::Read(buf)])
    }

    fn write(&mut self, reg: u8, value: u8) -> Result<(), S::Error> {
        self.spi.write(&[reg, value])
    }
}
// ANCHOR_END: regs

// ANCHOR: driver
/// Драйвер TMP-9. Логика одна, шина — параметр типа.
pub struct Tmp9<R> {
    regs: R,
}

impl<R: Regs> Tmp9<R> {
    /// Проверить, что на шине TMP-9, и запустить измерения.
    fn new(mut regs: R) -> Result<Self, Error<R::Error>> {
        let mut id = [0];
        regs.read(WHO_AM_I, &mut id)?;
        if id[0] != ID {
            return Err(Error::WrongId(id[0]));
        }
        regs.write(CTRL, CTRL_RUN)?;
        Ok(Tmp9 { regs })
    }

    /// Температура в сотых долях градуса Цельсия.
    pub fn temperature(&mut self) -> Result<i16, Error<R::Error>> {
        for _ in 0..POLLS {
            let mut status = [0];
            self.regs.read(STATUS, &mut status)?;
            if status[0] & STATUS_DRDY != 0 {
                // Оба байта одной транзакцией: иначе младший и старший
                // могут оказаться от разных измерений.
                let mut t = [0; 2];
                self.regs.read(TEMP_L, &mut t)?;
                return Ok(i16::from_le_bytes(t));
            }
        }
        Err(Error::Timeout)
    }
}

impl<I: I2c> Tmp9<I2cRegs<I>> {
    pub fn new_i2c(i2c: I, addr: u8) -> Result<Self, Error<I::Error>> {
        Tmp9::new(I2cRegs { i2c, addr })
    }

    /// Вернуть шину.
    pub fn release(self) -> I {
        self.regs.i2c
    }
}

impl<S: SpiDevice> Tmp9<SpiRegs<S>> {
    pub fn new_spi(spi: S) -> Result<Self, Error<S::Error>> {
        Tmp9::new(SpiRegs { spi })
    }

    /// Вернуть шину.
    pub fn release(self) -> S {
        self.regs.spi
    }
}
// ANCHOR_END: driver
