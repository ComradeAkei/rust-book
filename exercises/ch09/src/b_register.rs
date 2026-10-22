//! Задача B: регистры UART PL011 как типы.
//!
//! Файл самодостаточен: тест собирает его ещё и отдельно, как крейт, и
//! дописывает в конец пробы. Не ссылайтесь из него на другие модули
//! крейта.

use core::marker::PhantomData;

/// Базовый адрес UART0 (LM3S6965, тот же PL011 в QEMU `lm3s6965evb`).
pub const BASE: usize = 0x4000_C000;

/// Регистр UART0.
///
/// # Safety
///
/// `ADDR` — адрес 32-разрядного регистра UART0 на этом кристалле, и
/// обращение по нему не затрагивает ничего, кроме самого UART0.
pub unsafe trait Register {
    const ADDR: usize;
    /// Значение после сброса.
    const RESET: u32;
}

/// Регистр можно читать.
pub trait Readable: Register {}

/// В регистр можно писать.
pub trait Writable: Register {}

/// Шина, через которую драйвер обращается к регистрам. На кристалле это
/// [`Mmio`], в тестах — журнал обращений.
pub trait Bus {
    /// # Safety
    ///
    /// `addr` — адрес регистра, который можно читать.
    unsafe fn read(&mut self, addr: usize) -> u32;

    /// # Safety
    ///
    /// `addr` — адрес регистра, в который можно писать.
    unsafe fn write(&mut self, addr: usize, value: u32);
}

/// Настоящая шина: обращения к памяти через `volatile`.
pub struct Mmio(());

impl Mmio {
    /// # Safety
    ///
    /// Программа работает на кристалле, где UART0 лежит по адресу
    /// [`BASE`], и других владельцев у UART0 нет.
    pub unsafe fn new() -> Mmio {
        Mmio(())
    }
}

impl Bus for Mmio {
    unsafe fn read(&mut self, addr: usize) -> u32 {
        // SAFETY: по контракту `Bus::read` и `Mmio::new`.
        unsafe { (addr as *const u32).read_volatile() }
    }

    unsafe fn write(&mut self, addr: usize, value: u32) {
        // SAFETY: по контракту `Bus::write` и `Mmio::new`.
        unsafe { (addr as *mut u32).write_volatile(value) }
    }
}

/// Данные: запись кладёт байт в FIFO передатчика, чтение берёт из FIFO
/// приёмника.
pub struct Dr;
/// Флаги, только чтение.
pub struct Fr;
/// Целая часть делителя частоты.
pub struct Ibrd;
/// Дробная часть делителя, 6 бит.
pub struct Fbrd;
/// Формат кадра.
pub struct Lcrh;
/// Управление.
pub struct Cr;
/// Сброс прерываний, только запись.
pub struct Icr;

// Реализуйте `Register`, `Readable` и `Writable` для регистров выше по
// таблице из условия задачи.

/// Доступ к регистру `R`. Значений этого типа не бывает: всё, что он
/// знает, записано в константах `R`.
pub struct Reg<R>(PhantomData<R>);

// Ограничения на `R` здесь нарочно не те, что нужны: `read` и
// `is_reset` должны быть только у читаемых регистров, `write` и `reset` —
// у записываемых, `modify` — у тех, что и то и другое. Невыровненный адрес
// — ошибка компиляции.
impl<R> Reg<R> {
    pub fn read<B: Bus>(bus: &mut B) -> u32 {
        todo!("прочитать R::ADDR через шину")
    }

    /// Регистр в том же состоянии, что после сброса.
    pub fn is_reset<B: Bus>(bus: &mut B) -> bool {
        todo!()
    }

    pub fn write<B: Bus>(bus: &mut B, value: u32) {
        todo!("записать value по R::ADDR через шину")
    }

    /// Записать значение после сброса.
    pub fn reset<B: Bus>(bus: &mut B) {
        todo!()
    }

    pub fn modify<B: Bus>(bus: &mut B, f: impl FnOnce(u32) -> u32) {
        todo!("прочитать, изменить, записать")
    }
}

/// FR.TXFF: FIFO передатчика полон.
pub const FR_TXFF: u32 = 1 << 5;
/// LCRH: 8 бит данных (WLEN = 0b11) и FIFO включены (FEN).
pub const LCRH_8N1_FIFO: u32 = 0b11 << 5 | 1 << 4;
/// CR: UARTEN, TXE и RXE.
pub const CR_ENABLE: u32 = 1 | 1 << 8 | 1 << 9;

/// Делитель не помещается в IBRD и FBRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadBaud;

/// Включить UART: 8 бит, без чётности, 1 стоп-бит, FIFO.
pub fn init<B: Bus>(bus: &mut B, clock: u32, baud: u32) -> Result<(), BadBaud> {
    todo!("делитель, проверка, пять записей в регистры")
}

/// Передать байт: дождаться места в FIFO и записать в DR.
pub fn write_byte<B: Bus>(bus: &mut B, byte: u8) {
    todo!()
}

/// Вернуть UART в состояние после сброса: сначала выключить, затем
/// скорость и формат кадра.
pub fn deinit<B: Bus>(bus: &mut B) {
    todo!()
}

/// Для проверки под Cortex-M0: 12 МГц, 115200 бод.
///
/// # Safety
///
/// Как у [`Mmio::new`].
#[unsafe(no_mangle)]
pub unsafe fn ch09b_init_115200() -> bool {
    // SAFETY: по контракту функции.
    let mut bus = unsafe { Mmio::new() };
    init(&mut bus, 12_000_000, 115_200).is_ok()
}

/// # Safety
///
/// Как у [`Mmio::new`], и UART0 уже включён.
#[unsafe(no_mangle)]
pub unsafe fn ch09b_write_byte(byte: u8) {
    // SAFETY: по контракту функции.
    let mut bus = unsafe { Mmio::new() };
    write_byte(&mut bus, byte);
}
