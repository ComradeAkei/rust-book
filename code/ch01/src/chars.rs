//! Раздел 1.5: `char` и `bool` и их инварианты.

// ANCHOR: char
/// `char` — скалярное значение Unicode: от 0 до 0x10FFFF без суррогатов
/// 0xD800..=0xDFFF. Из произвольного `u32` его можно получить только через
/// проверку.
pub fn scalar(code: u32) -> Option<char> {
    char::from_u32(code)
}

/// Сколько байт займёт символ в UTF-8 и сами эти байты.
pub fn utf8(c: char) -> ([u8; 4], usize) {
    let mut buf = [0u8; 4];
    let len = c.encode_utf8(&mut buf).len();
    (buf, len)
}
// ANCHOR_END: char

// ANCHOR: bool
/// `bool` занимает байт, но допустимы только 0 и 1. Из байта его получают
/// сравнением, а не приведением.
pub fn flag(byte: u8) -> bool {
    byte != 0
}
// ANCHOR_END: bool

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn char_is_a_scalar_value() {
        assert_eq!(size_of::<char>(), 4);
        // Недопустимые значения char — ниша, Option<char> бесплатен.
        assert_eq!(size_of::<Option<char>>(), 4);
        assert_eq!(scalar(0x44F), Some('я'));
        assert_eq!(scalar(0xD800), None);
        assert_eq!(scalar(0x11_0000), None);
        assert_eq!(char::MAX as u32, 0x10_FFFF);
        assert_eq!('я' as u32, 0x44F);
        assert_eq!(0xE9u8 as char, 'é'); // u8 → char: Latin-1
    }

    #[test]
    fn utf8_lengths() {
        assert_eq!(utf8('A'), ([0x41, 0, 0, 0], 1));
        assert_eq!(utf8('я'), ([0xD1, 0x8F, 0, 0], 2));
        assert_eq!(utf8('€'), ([0xE2, 0x82, 0xAC, 0], 3));
        assert_eq!(utf8('🦀'), ([0xF0, 0x9F, 0xA6, 0x80], 4));
    }

    #[test]
    fn bool_is_one_byte() {
        assert_eq!(size_of::<bool>(), 1);
        assert_eq!(size_of::<Option<bool>>(), 1);
        assert_eq!(true as u8, 1);
        assert!(flag(2));
        assert_eq!(u8::from(flag(2)), 1);
    }
}
