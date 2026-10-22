//! Задача D: драйвер SPI-flash серии W25Q (Winbond) с состоянием в типе.
//!
//! Четыре состояния: `Sleep` — глубокий сон (команда 0xB9), `Ready` —
//! ожидание, `WriteEnabled` — после 0x06 установлен бит WEL, `Busy` — идёт
//! запись или стирание. В заготовке все методы доступны в любом состоянии,
//! а трейт `State` может реализовать кто угодно. Разнесите методы по
//! состояниям, запечатайте `State` и напишите тела.

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
/// Состояние микросхемы. Пока не запечатан.
pub trait State {}

pub enum Sleep {}
pub enum Ready {}
pub enum WriteEnabled {}
pub enum Busy {}

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

// ANCHOR: methods
impl<SPI: SpiDevice> Flash<SPI, Ready> {
    /// Подключиться: 0xAB, пауза `T_SLEEP_US`, 0x9F и проверка
    /// производителя, 0x05 и проверка, что BUSY сброшен.
    pub fn new(spi: SPI, delay: &mut impl DelayNs) -> Result<Self, (SPI, Error<SPI::Error>)> {
        todo!()
    }
}

/// Все методы во всех состояниях: компилятор пока ничего не запрещает.
impl<SPI: SpiDevice, S: State> Flash<SPI, S> {
    /// 0x03, адрес, чтение `buf.len()` байт. Состояние не меняется.
    pub fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Error<SPI::Error>> {
        todo!()
    }

    /// 0x06, затем 0x05 и проверка WEL.
    pub fn write_enable(self) -> Step<SPI, S, WriteEnabled> {
        todo!()
    }

    /// 0xB9, пауза `T_SLEEP_US`.
    pub fn sleep(self, delay: &mut impl DelayNs) -> Step<SPI, S, Sleep> {
        todo!()
    }

    /// 0x02, адрес, данные. Пустые данные и пересечение границы страницы —
    /// `PageOverflow` без обмена по шине.
    pub fn program(self, addr: u32, data: &[u8]) -> Step<SPI, S, Busy> {
        todo!()
    }

    /// 0x20, адрес. Адрес не кратен 4096 — `Unaligned` без обмена.
    pub fn erase_sector(self, addr: u32) -> Step<SPI, S, Busy> {
        todo!()
    }

    /// 0x04.
    pub fn disable(self) -> Step<SPI, S, Ready> {
        todo!()
    }

    /// Один опрос: 0x05 и проверка BUSY.
    pub fn poll(self) -> Result<Poll<SPI>, Failed<SPI, S>> {
        todo!()
    }

    /// Опрашивать до `max_polls` раз, потом `Timeout`.
    pub fn wait(self, max_polls: u32) -> Step<SPI, S, Ready> {
        todo!()
    }

    /// 0xAB, пауза `T_SLEEP_US`.
    pub fn wake(self, delay: &mut impl DelayNs) -> Step<SPI, S, Ready> {
        todo!()
    }

    /// Вернуть шину.
    pub fn free(self) -> SPI {
        self.spi
    }
}
// ANCHOR_END: methods

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

/// Полный цикл записи страницы: разрешить запись, записать, дождаться.
/// При ошибке вернуть драйвер в том состоянии, в каком его застала ошибка.
pub fn write_page<SPI: SpiDevice>(
    flash: Flash<SPI, Ready>,
    addr: u32,
    data: &[u8],
    max_polls: u32,
) -> PageResult<SPI> {
    todo!()
}
