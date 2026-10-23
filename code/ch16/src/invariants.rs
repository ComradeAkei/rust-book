//! Раздел 16.2: два вида инвариантов. Инвариант *валидности* — то, чему
//! значение обязано удовлетворять всегда, иначе само его существование —
//! UB (например, `bool` — это строго 0 или 1). Инвариант *безопасности* —
//! то, на что опирается безопасный код через `unsafe`; нарушить его можно
//! без немедленного UB, но тогда ломается тот самый безопасный код.

// ANCHOR: validity
/// Валидность: `bool` может быть только 0 или 1. Создать `bool` из байта
/// 2 — мгновенное UB, даже если его не использовать. Это и ловит Miri в
/// задаче A. Корректный разбор «байт в bool» идёт через сравнение, а не
/// через каламбур типов.
pub fn bool_from_byte(b: u8) -> Option<bool> {
    match b {
        0 => Some(false),
        1 => Some(true),
        _ => None, // не притворяемся, что 2 — это bool
    }
}
// ANCHOR_END: validity

// ANCHOR: safety
/// Тип с инвариантом безопасности: строка из одних ASCII-байт. Безопасный
/// метод `first_upper` опирается на это, чтобы резать по одному байту без
/// проверки границ символа. Инвариант держит `unsafe`-конструктор, а не
/// система типов, поэтому граница доверия — весь модуль.
pub struct Ascii(String);

impl Ascii {
    /// Проверяет инвариант и строит значение. Ошибка — если не ASCII.
    pub fn new(s: String) -> Result<Self, String> {
        if s.is_ascii() { Ok(Ascii(s)) } else { Err(s) }
    }

    /// Строит, не проверяя.
    ///
    /// # Safety
    ///
    /// Все байты `s` — ASCII (меньше 128).
    pub unsafe fn new_unchecked(s: String) -> Self {
        debug_assert!(s.is_ascii());
        Ascii(s)
    }

    /// Первый символ в верхнем регистре. Опирается на инвариант: раз все
    /// байты ASCII, байт 0 — это целый символ, и `[..1]` всегда на границе.
    pub fn first_upper(&self) -> Option<char> {
        self.0.bytes().next().map(|b| b.to_ascii_uppercase() as char)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
// ANCHOR_END: safety

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity_parses_not_puns() {
        assert_eq!(bool_from_byte(0), Some(false));
        assert_eq!(bool_from_byte(1), Some(true));
        assert_eq!(bool_from_byte(2), None);
    }

    #[test]
    fn safety_invariant_held() {
        let a = Ascii::new("adc0".to_string()).unwrap();
        assert_eq!(a.first_upper(), Some('A'));
        assert_eq!(a.as_str(), "adc0");
        assert!(Ascii::new("датчик".to_string()).is_err());
        // SAFETY: строка из одних ASCII-байт.
        let b = unsafe { Ascii::new_unchecked("i2c".to_string()) };
        assert_eq!(b.first_upper(), Some('I'));
    }
}
