//! Задача D: драйвер SPI-flash серии W25Q (Winbond) с состоянием в типе.
//!
//! Четыре состояния: `Sleep` — глубокий сон (команда 0xB9), `Ready` —
//! ожидание, `WriteEnabled` — после 0x06 установлен бит WEL, `Busy` — идёт
//! запись или стирание. Каждая команда — метод только того состояния, в
//! котором она допустима, и переход возвращает драйвер в новом типе.

use core::fmt;
use core::marker::PhantomData;
use embedded_hal::delay::DelayNs;
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};

// ANCHOR: commands
pub const CMD_WRITE_ENABLE: u8 = 0x06;
pub const CMD_WRITE_DISABLE: u8 = 0x04;
pub const CMD_READ_STATUS: u8 = 0x05;
pub const CMD_READ: u8 = 0x03;
pub const CMD_PAGE_PROGRAM: u8 = 0x02;
pub const CMD_SECTOR_ERASE: u8 = 0x20;
pub const CMD_POWER_DOWN: u8 = 0xB9;
pub const CMD_RELEASE: u8 = 0xAB;
pub const CMD_JEDEC_ID: u8 = 0x9F;

/// Регистр состояния 1: BUSY — идёт запись или стирание, WEL — запись
/// разрешена.
pub const STATUS_BUSY: u8 = 1 << 0;
pub const STATUS_WEL: u8 = 1 << 1;

pub const MANUFACTURER_WINBOND: u8 = 0xEF;
/// Страница программирования и сектор стирания.
pub const PAGE: u32 = 256;
pub const SECTOR: u32 = 4096;
/// Адрес — 24 бита: W25Q128 и меньшие.
pub const CAPACITY: u32 = 1 << 24;
/// tDP и tRES1 по документации W25Q128JV: 3 мкс до и после сна.
pub const T_SLEEP_US: u32 = 3;
// ANCHOR_END: commands

// ANCHOR: states
mod sealed {
    pub trait Sealed {}
}

/// Состояние микросхемы. Запечатан: других состояний нет.
pub trait State: sealed::Sealed {}

pub enum Sleep {}
pub enum Ready {}
pub enum WriteEnabled {}
pub enum Busy {}

impl sealed::Sealed for Sleep {}
impl sealed::Sealed for Ready {}
impl sealed::Sealed for WriteEnabled {}
impl sealed::Sealed for Busy {}
impl State for Sleep {}
impl State for Ready {}
impl State for WriteEnabled {}
impl State for Busy {}

/// Драйвер в состоянии `S`. Во время работы — только шина.
pub struct Flash<SPI, S: State> {
    spi: SPI,
    _state: PhantomData<S>,
}
// ANCHOR_END: states

/// Печатает только состояние: шине `Debug` не нужен.
impl<SPI, S: State> fmt::Debug for Flash<SPI, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = core::any::type_name::<S>();
        write!(f, "Flash<{}>", name.rsplit("::").next().unwrap_or(name))
    }
}

// ANCHOR: errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<E> {
    /// Ошибка шины.
    Spi(E),
    /// JEDEC ID не от Winbond.
    WrongId([u8; 3]),
    /// При подключении микросхема ещё пишет или стирает.
    StillBusy,
    /// После 0x06 бит WEL не установился.
    WriteNotEnabled,
    /// Данные пусты, длиннее страницы или пересекают её границу.
    PageOverflow,
    /// Адрес сектора не кратен 4096.
    Unaligned,
    /// Адрес за пределами 24 бит.
    OutOfRange,
    /// Запись не кончилась за отведённое число опросов.
    Timeout,
}

/// Неудачный переход: ошибка и драйвер в прежнем состоянии. Без этого
/// ошибка отняла бы у вызывающего и драйвер, и шину.
pub struct Failed<SPI: ErrorType, S: State> {
    pub flash: Flash<SPI, S>,
    pub error: Error<SPI::Error>,
}

impl<SPI: ErrorType, S: State> fmt::Debug for Failed<SPI, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Failed").field("error", &self.error).finish_non_exhaustive()
    }
}

/// Результат перехода из `From` в `To`.
pub type Step<SPI, From, To> = Result<Flash<SPI, To>, Failed<SPI, From>>;

/// Итог полного цикла записи: драйвер в `Ready` или ошибка и драйвер в
/// том состоянии, где она случилась.
pub type PageResult<SPI> =
    Result<Flash<SPI, Ready>, (AnyFlash<SPI>, Error<<SPI as ErrorType>::Error>)>;

/// Итог одного опроса занятой микросхемы.
pub enum Poll<SPI> {
    Done(Flash<SPI, Ready>),
    Pending(Flash<SPI, Busy>),
}
// ANCHOR_END: errors

fn cmd_addr(cmd: u8, addr: u32) -> [u8; 4] {
    let [_, a2, a1, a0] = addr.to_be_bytes();
    [cmd, a2, a1, a0]
}

// ANCHOR: common
impl<SPI: SpiDevice, S: State> Flash<SPI, S> {
    fn to<T: State>(self) -> Flash<SPI, T> {
        Flash { spi: self.spi, _state: PhantomData }
    }

    fn fail(self, error: Error<SPI::Error>) -> Failed<SPI, S> {
        Failed { flash: self, error }
    }

    /// Одна команда без данных, переход в `T` при успехе.
    fn command<T: State>(mut self, cmd: u8) -> Step<SPI, S, T> {
        match self.spi.write(&[cmd]) {
            Ok(()) => Ok(self.to()),
            Err(e) => Err(self.fail(Error::Spi(e))),
        }
    }

    fn status(&mut self) -> Result<u8, SPI::Error> {
        let mut s = [0];
        self.spi
            .transaction(&mut [Operation::Write(&[CMD_READ_STATUS]), Operation::Read(&mut s)])?;
        Ok(s[0])
    }

    /// Вернуть шину. Микросхема остаётся в том состоянии, в каком была.
    pub fn free(self) -> SPI {
        self.spi
    }
}
// ANCHOR_END: common

// ANCHOR: ready
impl<SPI: SpiDevice> Flash<SPI, Ready> {
    /// Подключиться: вывести из сна (0xAB бодрствующей микросхеме не
    /// мешает), проверить производителя и что запись не идёт.
    pub fn new(spi: SPI, delay: &mut impl DelayNs) -> Result<Self, (SPI, Error<SPI::Error>)> {
        let mut f = Flash { spi, _state: PhantomData };
        if let Err(e) = f.spi.write(&[CMD_RELEASE]) {
            return Err((f.spi, Error::Spi(e)));
        }
        delay.delay_us(T_SLEEP_US);
        let mut id = [0; 3];
        let checked = f
            .spi
            .transaction(&mut [Operation::Write(&[CMD_JEDEC_ID]), Operation::Read(&mut id)])
            .map_err(Error::Spi)
            .and_then(
                |()| if id[0] == MANUFACTURER_WINBOND { Ok(()) } else { Err(Error::WrongId(id)) },
            )
            .and_then(|()| f.status().map_err(Error::Spi))
            .and_then(|s| if s & STATUS_BUSY == 0 { Ok(()) } else { Err(Error::StillBusy) });
        match checked {
            Ok(()) => Ok(f),
            Err(e) => Err((f.spi, e)),
        }
    }

    /// Прочитать `buf.len()` байт с адреса `addr`. Состояние не меняется.
    pub fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Error<SPI::Error>> {
        if u64::from(addr) + buf.len() as u64 > u64::from(CAPACITY) {
            return Err(Error::OutOfRange);
        }
        self.spi
            .transaction(&mut [Operation::Write(&cmd_addr(CMD_READ, addr)), Operation::Read(buf)])
            .map_err(Error::Spi)
    }

    /// 0x06 и проверка, что WEL установился.
    pub fn write_enable(self) -> Step<SPI, Ready, WriteEnabled> {
        let mut f = self.command::<WriteEnabled>(CMD_WRITE_ENABLE)?;
        match f.status() {
            Ok(s) if s & STATUS_WEL != 0 => Ok(f),
            Ok(_) => Err(f.to().fail(Error::WriteNotEnabled)),
            Err(e) => Err(f.to().fail(Error::Spi(e))),
        }
    }

    /// Глубокий сон: ток покоя падает с десятков микроампер до единиц.
    pub fn sleep(self, delay: &mut impl DelayNs) -> Step<SPI, Ready, Sleep> {
        let f = self.command(CMD_POWER_DOWN)?;
        delay.delay_us(T_SLEEP_US);
        Ok(f)
    }
}
// ANCHOR_END: ready

// ANCHOR: write-enabled
impl<SPI: SpiDevice> Flash<SPI, WriteEnabled> {
    /// Записать до 256 байт в пределах одной страницы. Проверка границы —
    /// до обмена: микросхема бы молча завернула хвост в начало страницы.
    pub fn program(mut self, addr: u32, data: &[u8]) -> Step<SPI, WriteEnabled, Busy> {
        let fits = !data.is_empty() && (addr % PAGE) as usize + data.len() <= PAGE as usize;
        if !fits {
            return Err(self.fail(Error::PageOverflow));
        }
        if addr >= CAPACITY {
            return Err(self.fail(Error::OutOfRange));
        }
        let ops =
            &mut [Operation::Write(&cmd_addr(CMD_PAGE_PROGRAM, addr)), Operation::Write(data)];
        match self.spi.transaction(ops) {
            Ok(()) => Ok(self.to()),
            Err(e) => Err(self.fail(Error::Spi(e))),
        }
    }

    /// Стереть сектор 4 КиБ, `addr` кратен 4096.
    pub fn erase_sector(mut self, addr: u32) -> Step<SPI, WriteEnabled, Busy> {
        if addr % SECTOR != 0 {
            return Err(self.fail(Error::Unaligned));
        }
        if addr >= CAPACITY {
            return Err(self.fail(Error::OutOfRange));
        }
        match self.spi.write(&cmd_addr(CMD_SECTOR_ERASE, addr)) {
            Ok(()) => Ok(self.to()),
            Err(e) => Err(self.fail(Error::Spi(e))),
        }
    }

    /// Передумать: 0x04 сбрасывает WEL.
    pub fn disable(self) -> Step<SPI, WriteEnabled, Ready> {
        self.command(CMD_WRITE_DISABLE)
    }
}
// ANCHOR_END: write-enabled

// ANCHOR: busy
impl<SPI: SpiDevice> Flash<SPI, Busy> {
    /// Один опрос регистра состояния.
    pub fn poll(mut self) -> Result<Poll<SPI>, Failed<SPI, Busy>> {
        match self.status() {
            Ok(s) if s & STATUS_BUSY == 0 => Ok(Poll::Done(self.to())),
            Ok(_) => Ok(Poll::Pending(self)),
            Err(e) => Err(self.fail(Error::Spi(e))),
        }
    }

    /// Опрашивать, пока запись не кончится, но не больше `max_polls` раз.
    pub fn wait(mut self, max_polls: u32) -> Step<SPI, Busy, Ready> {
        for _ in 0..max_polls {
            self = match self.poll()? {
                Poll::Done(ready) => return Ok(ready),
                Poll::Pending(busy) => busy,
            };
        }
        Err(self.fail(Error::Timeout))
    }
}
// ANCHOR_END: busy

// ANCHOR: sleep
impl<SPI: SpiDevice> Flash<SPI, Sleep> {
    pub fn wake(self, delay: &mut impl DelayNs) -> Step<SPI, Sleep, Ready> {
        let f = self.command(CMD_RELEASE)?;
        delay.delay_us(T_SLEEP_US);
        Ok(f)
    }
}
// ANCHOR_END: sleep

// ANCHOR: any
/// Состояние, известное только во время работы, — например, драйвер в поле
/// структуры, которая живёт дольше одной функции. Typestate такого не
/// умеет: тип поля один. Приходится заворачивать в перечисление.
pub enum AnyFlash<SPI> {
    Sleep(Flash<SPI, Sleep>),
    Ready(Flash<SPI, Ready>),
    WriteEnabled(Flash<SPI, WriteEnabled>),
    Busy(Flash<SPI, Busy>),
}

impl<SPI> AnyFlash<SPI> {
    pub fn state(&self) -> &'static str {
        match self {
            AnyFlash::Sleep(_) => "Sleep",
            AnyFlash::Ready(_) => "Ready",
            AnyFlash::WriteEnabled(_) => "WriteEnabled",
            AnyFlash::Busy(_) => "Busy",
        }
    }
}

impl<SPI> fmt::Debug for AnyFlash<SPI> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyFlash::{}", self.state())
    }
}

impl<SPI> From<Poll<SPI>> for AnyFlash<SPI> {
    fn from(p: Poll<SPI>) -> Self {
        match p {
            Poll::Done(f) => AnyFlash::Ready(f),
            Poll::Pending(f) => AnyFlash::Busy(f),
        }
    }
}
// ANCHOR_END: any

// ANCHOR: write-page
/// Полный цикл записи страницы: разрешить запись, записать, дождаться.
/// При ошибке драйвер возвращается в том состоянии, в каком его застала
/// ошибка, — вызывающий решает, что с ним делать.
pub fn write_page<SPI: SpiDevice>(
    flash: Flash<SPI, Ready>,
    addr: u32,
    data: &[u8],
    max_polls: u32,
) -> PageResult<SPI> {
    let we = flash.write_enable().map_err(|f| (AnyFlash::Ready(f.flash), f.error))?;
    let busy = we.program(addr, data).map_err(|f| (AnyFlash::WriteEnabled(f.flash), f.error))?;
    busy.wait(max_polls).map_err(|f| (AnyFlash::Busy(f.flash), f.error))
}
// ANCHOR_END: write-page
