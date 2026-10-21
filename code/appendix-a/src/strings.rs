//! Раздел A.6: строки и форматированный вывод.

use core::fmt;

// ANCHOR: fixed_buf
/// Аналог `snprintf` в буфер фиксированного размера: без кучи, работает и
/// в `no_std`. Фрагмент, который не помещается, не пишется вовсе, и
/// `write!` возвращает ошибку вместо молчаливого усечения.
pub struct FixedBuf<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> FixedBuf<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, len: 0 }
    }

    pub fn as_str(&self) -> &str {
        // В буфер попадают только целые строки из `write_str`, поэтому
        // байты всегда образуют корректный UTF-8.
        core::str::from_utf8(&self.buf[..self.len]).expect("в буфер пишутся только целые &str")
    }
}

impl fmt::Write for FixedBuf<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        let dst = self.buf.get_mut(self.len..end).ok_or(fmt::Error)?;
        dst.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
// ANCHOR_END: fixed_buf

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;
    use std::ffi::CStr;

    #[test]
    fn snprintf_like() {
        let mut raw = [0u8; 12];
        let mut b = FixedBuf::new(&mut raw);
        write!(b, "t={:>5}", 42).unwrap();
        assert_eq!(b.as_str(), "t=   42");
        // Не помещается: ошибка, а написанное раньше осталось.
        assert!(write!(b, " overflow").is_err());
        assert_eq!(b.as_str(), "t=   42");
        // Аргументы пишутся по частям: «, » поместилось, число нет.
        // Литерал вместо `n` компилятор вклеил бы в строку формата целиком.
        let n = 123_456;
        assert!(write!(b, ", {n}").is_err());
        assert_eq!(b.as_str(), "t=   42, ");
    }

    // ANCHOR: printf_map
    #[test]
    fn printf_formats() {
        // Те же строки печатает c/printf.c.
        assert_eq!(format!("[{:5}|{:<5}|{:05}|{:+}]", 42, 42, 42, 5), "[   42|42   |00042|+5]");
        assert_eq!(
            format!("[{:x}|{:X}|{:#x}|{:08x}|{:#010x}|{:o}]", 255, 255, 255, 255, 255, 8),
            "[ff|FF|0xff|000000ff|0x000000ff|10]"
        );
        assert_eq!(
            format!("[{:.3}|{:>10}|{:<10}|{:.3}]", 1.23456, "abcdef", "abcdef", "abcdef"),
            "[1.235|    abcdef|abcdef    |abc]"
        );
        assert_eq!(format!("[{:8.2}|{}|%|{{}}]", 1.23456, 'A'), "[    1.23|A|%|{}]");
    }

    #[test]
    fn printf_differences() {
        // Строки по умолчанию выравниваются влево, числа вправо.
        assert_eq!(format!("[{:10}]", "abcdef"), "[abcdef    ]");
        assert_eq!(format!("[{:10}]", 42), "[        42]");
        // Экспонента без знака и ведущих нулей, мантисса кратчайшая.
        assert_eq!(format!("{:e}", 1.23456), "1.23456e0");
        assert_eq!(format!("{:.6e}", 1.23456), "1.234560e0");
        // Двоичный вывод, которого в printf до C23 не было.
        assert_eq!(format!("{:#010b}", 5u8), "0b00000101");
    }
    // ANCHOR_END: printf_map

    // ANCHOR: parse
    #[test]
    fn parse_numbers() {
        // atoi("42abc") == 42, atoi("abc") == 0: ошибку не отличить от нуля.
        assert_eq!("42".parse::<i32>(), Ok(42));
        assert!("42abc".parse::<i32>().is_err());
        assert!("abc".parse::<i32>().is_err());
        // strtol("0xff", NULL, 16) == 255, а from_str_radix префикс не понимает.
        assert_eq!(i64::from_str_radix("ff", 16), Ok(255));
        assert!(i64::from_str_radix("0xff", 16).is_err());
        let s = "0xff";
        assert_eq!(s.strip_prefix("0x").map(|h| i64::from_str_radix(h, 16)), Some(Ok(255)));
    }
    // ANCHOR_END: parse

    // ANCHOR: utf8
    #[test]
    fn utf8_lengths() {
        let s = "привет";
        assert_eq!(s.len(), 12); // байты, как strlen
        assert_eq!(s.chars().count(), 6); // символы Unicode
        assert_eq!(s.as_bytes()[0], 0xD0); // доступ к байту
        assert_eq!(s.get(0..1), None); // середина символа
        assert_eq!(&s[0..2], "п");
    }

    #[test]
    #[should_panic(expected = "byte index 1 is not a char boundary")]
    fn slicing_inside_char_panics() {
        let s = "привет";
        let _ = &s[0..1];
    }

    #[test]
    fn c_strings() {
        let c = c"abc"; // &'static CStr, NUL дописан компилятором
        assert_eq!(c.count_bytes(), 3);
        assert_eq!(c.to_bytes_with_nul(), b"abc\0");
        assert!(CStr::from_bytes_with_nul(&[b'a', 0, b'c', 0]).is_err()); // NUL внутри
        assert_eq!(CStr::from_bytes_until_nul(b"ab\0c").unwrap(), c"ab");
    }
    // ANCHOR_END: utf8
}
