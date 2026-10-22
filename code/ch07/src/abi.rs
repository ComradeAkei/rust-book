//! Раздел 7.1, «Под капотом»: как `Result` возвращается из функции на
//! x86_64 и Cortex-M. Четыре функции различаются только типом результата.

use core::num::NonZeroI32;

// ANCHOR: types
/// Байт, который не является шестнадцатеричной цифрой.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadHex(pub u8);

/// Переполнение при масштабировании отсчёта. Тип нулевого размера.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overflow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivError {
    ByZero,
    Inexact,
}

/// Код ошибки в стиле errno: ноль означает успех и поэтому недопустим.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Errno(pub NonZeroI32);
// ANCHOR_END: types

// ANCHOR: functions
/// `Result<u8, BadHex>`: 2 байта.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_hex(b: u8) -> Result<u8, BadHex> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(BadHex(b)),
    }
}

/// `Result<u32, Overflow>`: 8 байт, пара (признак, значение).
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_scale(raw: u32, gain: u32) -> Result<u32, Overflow> {
    raw.checked_mul(gain).ok_or(Overflow)
}

/// `Result<u64, DivError>`: 16 байт.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_div_exact(a: u64, b: u64) -> Result<u64, DivError> {
    if b == 0 {
        return Err(DivError::ByZero);
    }
    if !a.is_multiple_of(b) {
        return Err(DivError::Inexact);
    }
    Ok(a / b)
}

/// `Result<(), Errno>`: 4 байта, ровно как код возврата `int` в C.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_set_baud(baud: u32) -> Result<(), Errno> {
    match baud {
        9600 | 19200 | 115200 => Ok(()),
        _ => Err(Errno(NonZeroI32::new(22).unwrap())), // EINVAL
    }
}
// ANCHOR_END: functions

// ANCHOR: track_caller
/// Раздел 7.5: `#[track_caller]` добавляет скрытый аргумент — место вызова.
#[track_caller]
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_reg(n: usize) -> usize {
    assert!(n < 16, "регистр вне окна");
    n
}
// ANCHOR_END: track_caller

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn values() {
        assert_eq!(ch07_hex(b'7'), Ok(7));
        assert_eq!(ch07_hex(b'c'), Ok(12));
        assert_eq!(ch07_hex(b'G'), Err(BadHex(b'G')));
        assert_eq!(ch07_scale(1000, 4), Ok(4000));
        assert_eq!(ch07_scale(u32::MAX, 2), Err(Overflow));
        assert_eq!(ch07_div_exact(12, 4), Ok(3));
        assert_eq!(ch07_div_exact(12, 5), Err(DivError::Inexact));
        assert_eq!(ch07_div_exact(12, 0), Err(DivError::ByZero));
        assert_eq!(ch07_set_baud(115200), Ok(()));
        assert_eq!(ch07_set_baud(1200).unwrap_err().0.get(), 22);
        assert_eq!(ch07_reg(15), 15);
    }

    /// Размеры одинаковы на x86_64 и Cortex-M: ни одно поле не зависит от
    /// ширины указателя. Раскладка repr(Rust) не обещана, поэтому под
    /// случайной раскладкой эти числа проверяются только как наблюдение.
    #[test]
    fn observed_sizes() {
        assert_eq!(size_of::<Result<u8, BadHex>>(), 2);
        assert_eq!(size_of::<Result<u32, Overflow>>(), 8);
        assert_eq!(size_of::<Result<u64, DivError>>(), 16);
        assert_eq!(size_of::<Result<(), Errno>>(), 4);
    }
}
