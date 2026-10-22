//! Задача B: регистры UART PL011 как типы.
//!
//! Файл самодостаточен: тест собирает его ещё и отдельно, как крейт, и
//! дописывает в конец пробы. Ссылок на другие модули крейта в нём нет.

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

// ANCHOR: registers
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

// SAFETY: адреса по карте регистров UART в LM3S6965 Datasheet (смещения
// те же, что у ARM PL011), все регистры 32-разрядные и принадлежат UART0.
unsafe impl Register for Dr {
    const ADDR: usize = BASE;
    const RESET: u32 = 0;
}
unsafe impl Register for Fr {
    const ADDR: usize = BASE + 0x018;
    // TXFE и RXFE: оба FIFO пусты.
    const RESET: u32 = 0x90;
}
unsafe impl Register for Ibrd {
    const ADDR: usize = BASE + 0x024;
    const RESET: u32 = 0;
}
unsafe impl Register for Fbrd {
    const ADDR: usize = BASE + 0x028;
    const RESET: u32 = 0;
}
unsafe impl Register for Lcrh {
    const ADDR: usize = BASE + 0x02C;
    const RESET: u32 = 0;
}
unsafe impl Register for Cr {
    const ADDR: usize = BASE + 0x030;
    // TXE и RXE включены, сам UART (UARTEN) — нет.
    const RESET: u32 = 0x300;
}
unsafe impl Register for Icr {
    const ADDR: usize = BASE + 0x044;
    const RESET: u32 = 0;
}

impl Readable for Dr {}
impl Writable for Dr {}
impl Readable for Fr {}
impl Readable for Ibrd {}
impl Writable for Ibrd {}
impl Readable for Fbrd {}
impl Writable for Fbrd {}
impl Readable for Lcrh {}
impl Writable for Lcrh {}
impl Readable for Cr {}
impl Writable for Cr {}
impl Writable for Icr {}
// ANCHOR_END: registers

// ANCHOR: reg
/// Доступ к регистру `R`. Значений этого типа не бывает: всё, что он
/// знает, записано в константах `R`.
pub struct Reg<R>(PhantomData<R>);

impl<R: Register> Reg<R> {
    /// Вычисляется при мономорфизации, отдельно для каждого `R`.
    const ALIGNED: () = assert!(R::ADDR % 4 == 0, "адрес регистра не выровнен на 4");
}

impl<R: Readable> Reg<R> {
    pub fn read<B: Bus>(bus: &mut B) -> u32 {
        let () = Self::ALIGNED;
        // SAFETY: `R: Readable`, адрес обещан `unsafe impl Register`.
        unsafe { bus.read(R::ADDR) }
    }

    /// Регистр в том же состоянии, что после сброса.
    pub fn is_reset<B: Bus>(bus: &mut B) -> bool {
        Reg::<R>::read(bus) == R::RESET
    }
}

impl<R: Writable> Reg<R> {
    pub fn write<B: Bus>(bus: &mut B, value: u32) {
        let () = Self::ALIGNED;
        // SAFETY: `R: Writable`, адрес обещан `unsafe impl Register`.
        unsafe { bus.write(R::ADDR, value) }
    }

    /// Записать значение после сброса.
    pub fn reset<B: Bus>(bus: &mut B) {
        Reg::<R>::write(bus, R::RESET);
    }
}

impl<R: Readable + Writable> Reg<R> {
    pub fn modify<B: Bus>(bus: &mut B, f: impl FnOnce(u32) -> u32) {
        let value = f(Reg::<R>::read(bus));
        Reg::<R>::write(bus, value);
    }
}
// ANCHOR_END: reg

/// FR.TXFF: FIFO передатчика полон.
pub const FR_TXFF: u32 = 1 << 5;
/// LCRH: 8 бит данных (WLEN = 0b11) и FIFO включены (FEN).
pub const LCRH_8N1_FIFO: u32 = 0b11 << 5 | 1 << 4;
/// CR: UARTEN, TXE и RXE.
pub const CR_ENABLE: u32 = 1 | 1 << 8 | 1 << 9;

/// Делитель не помещается в IBRD и FBRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadBaud;

// ANCHOR: init
/// Включить UART: 8 бит, без чётности, 1 стоп-бит, FIFO.
pub fn init<B: Bus>(bus: &mut B, clock: u32, baud: u32) -> Result<(), BadBaud> {
    if baud == 0 {
        return Err(BadBaud);
    }
    // Делитель clock / (16 · baud) с шестью двоичными знаками после
    // запятой и округлением: 64 · clock / (16 · baud) = 4 · clock / baud.
    // В u64: 4 · clock в u32 переполняется уже на 1,07 ГГц.
    let div = (4 * u64::from(clock) + u64::from(baud / 2)) / u64::from(baud);
    let (ibrd, fbrd) = (div >> 6, div & 63);
    if ibrd == 0 || ibrd > 0xffff || (ibrd == 0xffff && fbrd != 0) {
        return Err(BadBaud);
    }
    // Порядок из Datasheet: выключить, задать скорость, затем формат кадра
    // (запись LCRH защёлкивает IBRD и FBRD), затем включить.
    Reg::<Cr>::write(bus, 0);
    Reg::<Ibrd>::write(bus, ibrd as u32);
    Reg::<Fbrd>::write(bus, fbrd as u32);
    Reg::<Lcrh>::write(bus, LCRH_8N1_FIFO);
    Reg::<Cr>::write(bus, CR_ENABLE);
    Ok(())
}

/// Передать байт: дождаться места в FIFO и записать в DR.
pub fn write_byte<B: Bus>(bus: &mut B, byte: u8) {
    while Reg::<Fr>::read(bus) & FR_TXFF != 0 {}
    Reg::<Dr>::write(bus, u32::from(byte));
}

/// Вернуть UART в состояние после сброса: сначала выключить, затем
/// скорость и формат кадра.
pub fn deinit<B: Bus>(bus: &mut B) {
    Reg::<Cr>::reset(bus);
    Reg::<Ibrd>::reset(bus);
    Reg::<Fbrd>::reset(bus);
    Reg::<Lcrh>::reset(bus);
}
// ANCHOR_END: init

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
