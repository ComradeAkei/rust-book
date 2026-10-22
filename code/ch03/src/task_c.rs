//! Задача 3.C: типы для ответов о передаче структур по значению.

// ANCHOR: thresholds
/// Пороги одного ABI на одной цели для структур
/// `#[repr(C)] struct W(pub [u32; N])`, в байтах.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds {
    /// Наибольший размер, при котором аргумент целиком передаётся в
    /// регистрах.
    pub arg_in_regs: usize,
    /// Наибольший размер, при котором результат целиком возвращается в
    /// регистрах.
    pub ret_in_regs: usize,
}
// ANCHOR_END: thresholds

/// Наибольшее число слов `u32` в структурах, которые проверяет тест.
pub const MAX_WORDS: usize = 48;
