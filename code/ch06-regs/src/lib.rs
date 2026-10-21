//! Блок регистров UART PL011 (машина QEMU `lm3s6965evb`, базовый адрес 0x4000_C000).
//!
//! Глава 6 использует этот крейт, чтобы показать три вещи:
//! раскладку `repr(C)` как карту памяти устройства, `repr(transparent)`-обёртку
//! над регистром и проверку смещений на этапе компиляции.
#![no_std]

use core::cell::UnsafeCell;
use core::mem::offset_of;

// ANCHOR: volatile
/// Ячейка регистра: читается и пишется только через volatile-доступ.
///
/// `UnsafeCell<T>` имеет ту же раскладку, что и `T`, а `repr(transparent)`
/// гарантирует, что и `Volatile<T>` занимает ровно столько же места.
#[repr(transparent)]
pub struct Volatile<T: Copy>(UnsafeCell<T>);

impl<T: Copy> Volatile<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    #[inline(always)]
    pub fn read(&self) -> T {
        // SAFETY: указатель из UnsafeCell валиден и выровнен, пока жив `self`.
        unsafe { self.0.get().read_volatile() }
    }

    #[inline(always)]
    pub fn write(&self, value: T) {
        // SAFETY: см. `read`; изменение через `&self` законно благодаря UnsafeCell.
        unsafe { self.0.get().write_volatile(value) }
    }
}
// ANCHOR_END: volatile

// ANCHOR: flags
/// Регистр флагов UARTFR. Биты по документации ARM PL011 TRM.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Flags(u32);

impl Flags {
    pub const BUSY: u32 = 1 << 3;
    pub const RXFE: u32 = 1 << 4;
    pub const TXFF: u32 = 1 << 5;
    pub const RXFF: u32 = 1 << 6;
    pub const TXFE: u32 = 1 << 7;

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
    pub const fn bits(self) -> u32 {
        self.0
    }
    /// Очередь передатчика заполнена: писать в DR пока нельзя.
    pub const fn tx_full(self) -> bool {
        self.0 & Self::TXFF != 0
    }
    /// Очередь приёмника пуста: читать из DR нечего.
    pub const fn rx_empty(self) -> bool {
        self.0 & Self::RXFE != 0
    }
    pub const fn busy(self) -> bool {
        self.0 & Self::BUSY != 0
    }
}
// ANCHOR_END: flags

// ANCHOR: block
/// Карта регистров PL011. Порядок и смещения полей совпадают с документацией,
/// потому что `repr(C)` запрещает компилятору переставлять поля.
#[repr(C)]
pub struct Pl011 {
    pub dr: Volatile<u32>,      // 0x000 данные
    pub rsr_ecr: Volatile<u32>, // 0x004 статус приёма и сброс ошибок
    _reserved0: [u32; 4],       // 0x008..0x018
    pub fr: Volatile<u32>,      // 0x018 флаги
    _reserved1: u32,            // 0x01C
    pub ilpr: Volatile<u32>,    // 0x020
    pub ibrd: Volatile<u32>,    // 0x024 целая часть делителя
    pub fbrd: Volatile<u32>,    // 0x028 дробная часть делителя
    pub lcr_h: Volatile<u32>,   // 0x02C формат кадра
    pub cr: Volatile<u32>,      // 0x030 управление
}

// Если кто-то случайно добавит или удалит поле, сборка сломается здесь,
// а не на плате через неделю отладки.
const _: () = {
    assert!(offset_of!(Pl011, dr) == 0x000);
    assert!(offset_of!(Pl011, rsr_ecr) == 0x004);
    assert!(offset_of!(Pl011, fr) == 0x018);
    assert!(offset_of!(Pl011, ibrd) == 0x024);
    assert!(offset_of!(Pl011, cr) == 0x030);
    assert!(core::mem::size_of::<Pl011>() == 0x034);
};
// ANCHOR_END: block

// ANCHOR: driver
impl Pl011 {
    /// Базовый адрес UART0 на машине QEMU lm3s6965evb.
    pub const UART0_BASE: usize = 0x4000_C000;

    /// # Safety
    /// По адресу `base` должен находиться блок регистров PL011, и никто
    /// другой не должен одновременно им пользоваться.
    pub unsafe fn at(base: usize) -> &'static Self {
        // SAFETY: обязанности вызывающего описаны выше.
        unsafe { &*(base as *const Self) }
    }

    pub fn flags(&self) -> Flags {
        Flags::from_bits(self.fr.read())
    }

    pub fn write_byte(&self, byte: u8) {
        while self.flags().tx_full() {
            core::hint::spin_loop();
        }
        self.dr.write(byte as u32);
    }

    pub fn write_str(&self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b);
        }
    }

    /// Блок регистров в обычной памяти: нужен только тестам на хосте.
    pub const fn zeroed() -> Self {
        Self {
            dr: Volatile::new(0),
            rsr_ecr: Volatile::new(0),
            _reserved0: [0; 4],
            fr: Volatile::new(0),
            _reserved1: 0,
            ilpr: Volatile::new(0),
            ibrd: Volatile::new(0),
            fbrd: Volatile::new(0),
            lcr_h: Volatile::new(0),
            cr: Volatile::new(0),
        }
    }
}
// ANCHOR_END: driver

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_decode() {
        let f = Flags::from_bits(Flags::TXFF | Flags::BUSY);
        assert!(f.tx_full());
        assert!(f.busy());
        assert!(!f.rx_empty());
    }

    #[test]
    fn write_byte_goes_to_dr() {
        let uart = Pl011::zeroed();
        uart.write_byte(b'A');
        assert_eq!(uart.dr.read(), b'A' as u32);
    }

    #[test]
    fn wrapper_is_free() {
        assert_eq!(core::mem::size_of::<Volatile<u32>>(), 4);
        assert_eq!(core::mem::size_of::<Flags>(), 4);
    }
}
