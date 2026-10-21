//! Задача 6.C. Регистр UARTLCR_H контроллера PL011 как тип-обёртка.
//!
//! ```text
//!  7     6..5   4     3     2     1     0
//! SPS   WLEN   FEN  STP2   EPS   PEN   BRK
//! ```
//!
//! 1. Реализуйте методы `LcrH`. Все они должны быть `const fn`, чтобы
//!    конфигурацию можно было проверить при компиляции через `const _`.
//! 2. Реализуйте `ch06c_configure_typed` и `ch06c_set_word_len_typed`.
//! 3. Докажите по ассемблеру, что обёртка бесплатна. Сравните пары функций:
//!    ```text
//!    cargo asm -p ch06-exercises --lib --target thumbv7m-none-eabi ch06c_configure_raw
//!    cargo asm -p ch06-exercises --lib --target thumbv7m-none-eabi ch06c_configure_typed
//!    ```
//!    и то же для `set_word_len`, затем для x86_64. В отчёте приведите
//!    листинги и объясните каждую инструкцию.
//!
//! Таблица чётности (PEN, SPS, EPS): PEN = 0 — нет чётности при любых SPS и
//! EPS; 1,0,0 — нечётность; 1,0,1 — чётность; 1,1,0 — бит всегда 1 (Mark);
//! 1,1,1 — бит всегда 0 (Space).

use ch06_regs::Pl011;

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
    Mark,
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

impl LcrH {
    /// Значение после сброса: 5 бит, без чётности, 1 стоповый, FIFO выключен.
    pub const RESET: Self = Self(0);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn word_len(self) -> WordLen {
        todo!()
    }
    /// Меняет только биты WLEN, остальные сохраняет.
    pub const fn with_word_len(self, w: WordLen) -> Self {
        todo!()
    }

    pub const fn fifo_enabled(self) -> bool {
        todo!()
    }
    pub const fn with_fifo(self, on: bool) -> Self {
        todo!()
    }

    pub const fn stop_bits(self) -> StopBits {
        todo!()
    }
    pub const fn with_stop_bits(self, s: StopBits) -> Self {
        todo!()
    }

    pub const fn sends_break(self) -> bool {
        todo!()
    }
    pub const fn with_break(self, on: bool) -> Self {
        todo!()
    }

    pub const fn parity(self) -> Parity {
        todo!()
    }
    /// Запись должна быть канонической: `Parity::None` сбрасывает PEN, EPS и SPS.
    pub const fn with_parity(self, p: Parity) -> Self {
        todo!()
    }
}

/// 8N1 с FIFO, записанное магическим числом, как в типичном C-драйвере.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_configure_raw(uart: &Pl011) {
    uart.lcr_h.write(0x70);
}

/// То же самое через `LcrH`: без единого числового литерала.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_configure_typed(uart: &Pl011) {
    todo!()
}

/// Чтение-изменение-запись поля WLEN масками.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_set_word_len_raw(uart: &Pl011, w: WordLen) {
    let v = uart.lcr_h.read();
    uart.lcr_h.write((v & !(0b11 << 5)) | ((w as u32) << 5));
}

/// То же через `LcrH`.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06c_set_word_len_typed(uart: &Pl011, w: WordLen) {
    todo!()
}
