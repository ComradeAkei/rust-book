//! Задача B: `#[derive(Bitfield)]`. Пример — формат кадра PL011, регистр
//! UARTLCRH: восемь полей в младших восьми битах.

// ANCHOR: lcrh
/// Формат кадра UART (UARTLCRH).
#[derive(crate::Bitfield, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lcrh {
    /// Послать разрыв линии.
    #[bits(1)]
    pub brk: bool,
    /// Чётность включена.
    #[bits(1)]
    pub pen: bool,
    /// Чётность — чётная.
    #[bits(1)]
    pub eps: bool,
    /// Два стоповых бита.
    #[bits(1)]
    pub stp2: bool,
    /// FIFO включены.
    #[bits(1)]
    pub fen: bool,
    /// Длина слова: 0 — 5 бит … 3 — 8 бит.
    #[bits(2)]
    pub wlen: u8,
    /// Фиксированная чётность.
    #[bits(1)]
    pub sps: bool,
}
// ANCHOR_END: lcrh

/// 8N1 с FIFO — так настраивает UART почти любая прошивка.
pub const LCRH_8N1_FIFO: LcrhBits = LcrhBits(0).with_wlen(3).with_fen(true);
