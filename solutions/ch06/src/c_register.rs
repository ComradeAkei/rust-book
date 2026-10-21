//! Задача 6.C: регистр UARTLCR_H контроллера PL011 как тип-обёртка.
//!
//! Раскладка битов по ARM PrimeCell UART (PL011) Technical Reference Manual:
//!
//! ```text
//!  7     6..5   4     3     2     1     0
//! SPS   WLEN   FEN  STP2   EPS   PEN   BRK
//! ```

use ch06_regs::Pl011;

// ANCHOR: types
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum WordLen {
    Five = 0b00,
    Six = 0b01,
    Seven = 0b10,
    Eight = 0b11,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parity {
    None,
    Odd,
    Even,
    /// Бит чётности всегда 1 (stick parity, SPS = 1, EPS = 0).
    Mark,
    /// Бит чётности всегда 0 (stick parity, SPS = 1, EPS = 1).
    Space,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StopBits {
    One,
    Two,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct LcrH(u32);
// ANCHOR_END: types

// ANCHOR: impl
impl LcrH {
    const BRK: u32 = 1 << 0;
    const PEN: u32 = 1 << 1;
    const EPS: u32 = 1 << 2;
    const STP2: u32 = 1 << 3;
    const FEN: u32 = 1 << 4;
    const WLEN_SHIFT: u32 = 5;
    const WLEN_MASK: u32 = 0b11 << Self::WLEN_SHIFT;
    const SPS: u32 = 1 << 7;
    const PARITY_MASK: u32 = Self::PEN | Self::EPS | Self::SPS;

    /// Значение после сброса: 5 бит, без чётности, 1 стоповый, FIFO выключен.
    pub const RESET: Self = Self(0);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn word_len(self) -> WordLen {
        match (self.0 & Self::WLEN_MASK) >> Self::WLEN_SHIFT {
            0b00 => WordLen::Five,
            0b01 => WordLen::Six,
            0b10 => WordLen::Seven,
            _ => WordLen::Eight,
        }
    }
    pub const fn with_word_len(self, w: WordLen) -> Self {
        Self((self.0 & !Self::WLEN_MASK) | ((w as u32) << Self::WLEN_SHIFT))
    }

    pub const fn fifo_enabled(self) -> bool {
        self.0 & Self::FEN != 0
    }
    pub const fn with_fifo(self, on: bool) -> Self {
        Self::set(self, Self::FEN, on)
    }

    pub const fn stop_bits(self) -> StopBits {
        if self.0 & Self::STP2 != 0 { StopBits::Two } else { StopBits::One }
    }
    pub const fn with_stop_bits(self, s: StopBits) -> Self {
        Self::set(self, Self::STP2, matches!(s, StopBits::Two))
    }

    pub const fn sends_break(self) -> bool {
        self.0 & Self::BRK != 0
    }
    pub const fn with_break(self, on: bool) -> Self {
        Self::set(self, Self::BRK, on)
    }

    /// Без PEN биты EPS и SPS ничего не значат, поэтому любая их комбинация
    /// при PEN = 0 читается как `Parity::None`.
    pub const fn parity(self) -> Parity {
        if self.0 & Self::PEN == 0 {
            return Parity::None;
        }
        match (self.0 & Self::SPS != 0, self.0 & Self::EPS != 0) {
            (false, false) => Parity::Odd,
            (false, true) => Parity::Even,
            (true, false) => Parity::Mark,
            (true, true) => Parity::Space,
        }
    }
    /// Запись всегда канонична: `Parity::None` сбрасывает и EPS, и SPS.
    pub const fn with_parity(self, p: Parity) -> Self {
        let bits = match p {
            Parity::None => 0,
            Parity::Odd => Self::PEN,
            Parity::Even => Self::PEN | Self::EPS,
            Parity::Mark => Self::PEN | Self::SPS,
            Parity::Space => Self::PEN | Self::SPS | Self::EPS,
        };
        Self((self.0 & !Self::PARITY_MASK) | bits)
    }

    const fn set(self, mask: u32, on: bool) -> Self {
        if on { Self(self.0 | mask) } else { Self(self.0 & !mask) }
    }
}

// Типизированная конфигурация вычисляется при компиляции и совпадает
// с «магическим числом» из документации.
const _: () = assert!(LcrH::RESET.with_word_len(WordLen::Eight).with_fifo(true).bits() == 0x70);
// ANCHOR_END: impl

// ANCHOR: configure
/// 8N1 с FIFO, записанное магическим числом, как в типичном C-драйвере.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_configure_raw(uart: &Pl011) {
    uart.lcr_h.write(0x70);
}

/// То же самое через типизированную обёртку.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_configure_typed(uart: &Pl011) {
    let v = LcrH::RESET.with_word_len(WordLen::Eight).with_fifo(true);
    uart.lcr_h.write(v.bits());
}

/// Чтение-изменение-запись одного поля: вручную масками.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_set_word_len_raw(uart: &Pl011, w: WordLen) {
    let v = uart.lcr_h.read();
    uart.lcr_h.write((v & !(0b11 << 5)) | ((w as u32) << 5));
}

/// Чтение-изменение-запись через обёртку.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_set_word_len_typed(uart: &Pl011, w: WordLen) {
    let v = LcrH::from_bits(uart.lcr_h.read());
    uart.lcr_h.write(v.with_word_len(w).bits());
}
// ANCHOR_END: configure
