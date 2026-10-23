//! Раздел 10.7: типы-маркеры и typestate.
//!
//! Физический адрес вместо виртуального не пройдёт: у них разные типы,
//! хотя внутри обоих один `usize`.
//!
//! ```compile_fail,E0308
//! use ch10::typestate::{Addr, Virt, dma_start};
//! let buf = Addr::<Virt>::new(0x8000_1000);
//! dma_start(buf, 64);
//! ```
//!
//! Передавать байты можно только через включённый UART:
//!
//! ```compile_fail,E0599
//! use ch06_regs::Pl011;
//! use ch10::typestate::Uart;
//! let regs = Pl011::zeroed();
//! let mut uart = Uart::new(&regs);
//! uart.write_byte(b'A');
//! ```
//!
//! Переход забирает старое состояние: после `enable` прежнего
//! `Uart<Disabled>` больше нет.
//!
//! ```compile_fail,E0382
//! use ch06_regs::Pl011;
//! use ch10::typestate::{BAUD_115200_12MHZ, Uart};
//! let regs = Pl011::zeroed();
//! let off = Uart::new(&regs);
//! let on = off.enable(BAUD_115200_12MHZ);
//! let again = off.enable(BAUD_115200_12MHZ);
//! ```
//!
//! Своё состояние снаружи не добавить: трейт `State` запечатан.
//!
//! ```compile_fail,E0277
//! enum Broken {}
//! impl ch10::typestate::State for Broken {}
//! ```

use ch06_regs::Pl011;
use core::fmt;
use core::marker::PhantomData;

// ANCHOR: addr
/// Адресное пространство — только на уровне типов: значений у маркеров
/// нет, и места они не занимают.
pub enum Phys {}
pub enum Virt {}

/// Адрес в пространстве `S`. Внутри — один `usize`.
pub struct Addr<S> {
    raw: usize,
    _space: PhantomData<S>,
}

// derive(Clone, Copy) потребовал бы S: Copy, а у пустых enum-маркеров
// значений нет вовсе. PhantomData<S> копируется при любом S.
impl<S> Clone for Addr<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Addr<S> {}

impl<S> Addr<S> {
    pub const fn new(raw: usize) -> Self {
        Addr { raw, _space: PhantomData }
    }

    pub const fn raw(self) -> usize {
        self.raw
    }
}

/// Окно трансляции: виртуальные адреса `virt..virt + len` отображены в
/// физические с `phys`.
pub struct Window {
    pub virt: usize,
    pub phys: usize,
    pub len: usize,
}

impl Window {
    pub fn translate(&self, va: Addr<Virt>) -> Option<Addr<Phys>> {
        let off = va.raw.checked_sub(self.virt).filter(|&o| o < self.len)?;
        Some(Addr::new(self.phys + off))
    }
}

/// Контроллер DMA видит только физическую память.
pub fn dma_start(buf: Addr<Phys>, len: usize) -> (usize, usize) {
    (buf.raw, len)
}
// ANCHOR_END: addr

// ANCHOR: states
mod sealed {
    pub trait Sealed {}
}

/// Состояние UART. Запечатан: других состояний, кроме двух ниже, нет.
pub trait State: sealed::Sealed {}

pub enum Disabled {}
pub enum Enabled {}

impl sealed::Sealed for Disabled {}
impl sealed::Sealed for Enabled {}
impl State for Disabled {}
impl State for Enabled {}

/// UART PL011 в состоянии `S`. Во время работы есть только ссылка на
/// регистры: состояние живёт в типе.
pub struct Uart<'a, S: State> {
    regs: &'a Pl011,
    _state: PhantomData<S>,
}
// ANCHOR_END: states

/// Делитель частоты: целая и дробная (в 1/64) части, как их пишут в IBRD и
/// FBRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divisor {
    pub ibrd: u16,
    pub fbrd: u8,
}

/// 115200 бод при такте 12 МГц: 12e6 / (16 · 115200) = 6,51 → 6 и 33/64.
pub const BAUD_115200_12MHZ: Divisor = Divisor { ibrd: 6, fbrd: 33 };

const LCRH_8N1_FIFO: u32 = 0x70;
const CR_UARTEN_TXE_RXE: u32 = 0x301;

// ANCHOR: transitions
impl<'a> Uart<'a, Disabled> {
    /// Взять UART и выключить его: дальше тип и железо согласны.
    pub fn new(regs: &'a Pl011) -> Self {
        regs.cr.write(0);
        Uart { regs, _state: PhantomData }
    }

    /// Настроить скорость и формат кадра и включить. Делитель и формат
    /// PL011 разрешает менять только при выключенном UART, и тип это
    /// гарантирует.
    pub fn enable(self, div: Divisor) -> Uart<'a, Enabled> {
        self.regs.ibrd.write(div.ibrd.into());
        self.regs.fbrd.write(div.fbrd.into());
        self.regs.lcr_h.write(LCRH_8N1_FIFO);
        self.regs.cr.write(CR_UARTEN_TXE_RXE);
        Uart { regs: self.regs, _state: PhantomData }
    }
}

impl<'a> Uart<'a, Enabled> {
    pub fn write_byte(&mut self, b: u8) {
        self.regs.write_byte(b);
    }

    /// Дождаться, пока уйдёт последний бит, и выключить.
    pub fn disable(self) -> Uart<'a, Disabled> {
        while self.regs.flags().busy() {
            core::hint::spin_loop();
        }
        self.regs.cr.write(0);
        Uart { regs: self.regs, _state: PhantomData }
    }
}

impl fmt::Write for Uart<'_, Enabled> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.regs.write_str(s);
        Ok(())
    }
}

/// Методы для любого состояния.
impl<'a, S: State> Uart<'a, S> {
    /// Вернуть регистры. UART остаётся в том состоянии, в каком был.
    pub fn release(self) -> &'a Pl011 {
        self.regs
    }
}
// ANCHOR_END: transitions

// ANCHOR: checked
/// То же с состоянием во время работы: флаг в структуре и проверка в
/// каждом методе.
pub struct UartChecked<'a> {
    regs: &'a Pl011,
    enabled: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NotEnabled;

impl<'a> UartChecked<'a> {
    pub fn new(regs: &'a Pl011) -> Self {
        regs.cr.write(0);
        UartChecked { regs, enabled: false }
    }

    pub fn enable(&mut self, div: Divisor) {
        if self.enabled {
            self.regs.cr.write(0);
        }
        self.regs.ibrd.write(div.ibrd.into());
        self.regs.fbrd.write(div.fbrd.into());
        self.regs.lcr_h.write(LCRH_8N1_FIFO);
        self.regs.cr.write(CR_UARTEN_TXE_RXE);
        self.enabled = true;
    }

    pub fn write_byte(&mut self, b: u8) -> Result<(), NotEnabled> {
        if !self.enabled {
            return Err(NotEnabled);
        }
        self.regs.write_byte(b);
        Ok(())
    }
}
// ANCHOR_END: checked

// ANCHOR: cost
/// Включить, передать байт, выключить — через typestate.
#[unsafe(no_mangle)]
pub fn ch10_typed_send(regs: &Pl011, b: u8) {
    let mut uart = Uart::new(regs).enable(BAUD_115200_12MHZ);
    uart.write_byte(b);
    uart.disable();
}

/// То же голыми записями в регистры.
#[unsafe(no_mangle)]
pub fn ch10_raw_send(regs: &Pl011, b: u8) {
    regs.cr.write(0);
    regs.ibrd.write(6);
    regs.fbrd.write(33);
    regs.lcr_h.write(LCRH_8N1_FIFO);
    regs.cr.write(CR_UARTEN_TXE_RXE);
    regs.write_byte(b);
    while regs.flags().busy() {
        core::hint::spin_loop();
    }
    regs.cr.write(0);
}

/// Передать байт через UART с проверкой во время работы.
#[unsafe(no_mangle)]
pub fn ch10_checked_write(uart: &mut UartChecked<'_>, b: u8) -> bool {
    uart.write_byte(b).is_ok()
}

/// Передать байт через уже включённый UART.
#[unsafe(no_mangle)]
pub fn ch10_typed_write(uart: &mut Uart<'_, Enabled>, b: u8) {
    uart.write_byte(b);
}
// ANCHOR_END: cost

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;

    #[test]
    fn address_spaces() {
        let w = Window { virt: 0x8000_0000, phys: 0x2000_0000, len: 0x1_0000 };
        let pa = w.translate(Addr::new(0x8000_1000)).unwrap();
        assert_eq!(dma_start(pa, 64), (0x2000_1000, 64));
        assert!(w.translate(Addr::new(0x8001_0000)).is_none());
        assert!(w.translate(Addr::new(0x7fff_ffff)).is_none());
        assert_eq!(size_of::<Addr<Phys>>(), size_of::<usize>());
        let copy = pa;
        assert_eq!(copy.raw(), pa.raw());
    }

    #[test]
    fn uart_states() {
        let regs = Pl011::zeroed();
        regs.cr.write(0xffff);
        let off = Uart::new(&regs);
        assert_eq!(regs.cr.read(), 0);
        let mut on = off.enable(BAUD_115200_12MHZ);
        assert_eq!((regs.ibrd.read(), regs.fbrd.read()), (6, 33));
        assert_eq!((regs.lcr_h.read(), regs.cr.read()), (0x70, 0x301));
        write!(on, "ok").unwrap();
        on.write_byte(b'!');
        assert_eq!(regs.dr.read(), u32::from(b'!'));
        let off = on.disable();
        assert_eq!(regs.cr.read(), 0);
        assert!(core::ptr::eq(off.release(), &regs));
    }

    #[test]
    fn zero_cost() {
        assert_eq!(size_of::<Uart<'_, Enabled>>(), size_of::<&Pl011>());
        assert_eq!(size_of::<Uart<'_, Disabled>>(), size_of::<&Pl011>());
        assert_eq!(size_of::<UartChecked<'_>>(), 2 * size_of::<&Pl011>());
        let regs = Pl011::zeroed();
        let mut c = UartChecked::new(&regs);
        assert!(!ch10_checked_write(&mut c, b'x'));
        c.enable(BAUD_115200_12MHZ);
        assert!(ch10_checked_write(&mut c, b'y'));
        assert_eq!(regs.dr.read(), u32::from(b'y'));
        ch10_typed_send(&regs, b'z');
        assert_eq!((regs.dr.read(), regs.cr.read()), (u32::from(b'z'), 0));
        ch10_raw_send(&regs, b'w');
        assert_eq!((regs.dr.read(), regs.cr.read()), (u32::from(b'w'), 0));
        let mut on = Uart::new(&regs).enable(BAUD_115200_12MHZ);
        ch10_typed_write(&mut on, b'v');
        assert_eq!(regs.dr.read(), u32::from(b'v'));
    }
}
