//! Раздел 1.6: массивы, срезы и строки.
//!
//! ```text
//! cargo asm -p ch01 --lib ch01_get
//! cargo asm -p ch01 --lib ch01_sum_indexed
//! cargo asm -p ch01 --lib ch01_sum_iter
//! ```

// ANCHOR: array
/// Массив — значение: передаётся и копируется целиком, к указателю не
/// сводится. Длина — часть типа.
pub fn reversed(mut a: [u8; 4]) -> [u8; 4] {
    a.reverse();
    a
}
// ANCHOR_END: array

// ANCHOR: index
/// Индексация среза проверяет границу: сравнение и переход на панику.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_get(xs: &[u32], i: usize) -> u32 {
    xs[i]
}
// ANCHOR_END: index

// ANCHOR: loops
/// Цикл по индексам: граница `0..xs.len()` доказывает компилятору, что
/// `xs[i]` не выйдет за срез, и проверка исчезает.
#[inline(never)]
#[unsafe(no_mangle)]
#[allow(clippy::needless_range_loop)]
pub fn ch01_sum_indexed(xs: &[u32]) -> u32 {
    let mut s = 0u32;
    for i in 0..xs.len() {
        s = s.wrapping_add(xs[i]);
    }
    s
}

/// Цикл по индексам с чужой границей: компилятор не знает, что `n` не
/// больше длины. Но тело цикла ничего не меняет снаружи, и единственную
/// проверку `n - 1 < len` он выносит перед циклом: паника случится раньше
/// суммирования, а не после, и снаружи этого не видно.
#[inline(never)]
#[unsafe(no_mangle)]
#[allow(clippy::needless_range_loop)]
pub fn ch01_sum_first(xs: &[u32], n: usize) -> u32 {
    let mut s = 0u32;
    for i in 0..n {
        s = s.wrapping_add(xs[i]);
    }
    s
}

/// Итератор не индексирует вовсе.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_sum_iter(xs: &[u32]) -> u32 {
    xs.iter().fold(0u32, |s, &x| s.wrapping_add(x))
}
// ANCHOR_END: loops

// ANCHOR: str
/// Первый символ строки. Индекс `&s[..1]` упал бы на любой букве вне ASCII:
/// длина `str` считается в байтах, а резать можно только по границе символа.
pub fn first_char(s: &str) -> Option<&str> {
    let c = s.chars().next()?;
    Some(&s[..c.len_utf8()])
}
// ANCHOR_END: str

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn arrays_and_slices() {
        assert_eq!(size_of::<[u16; 5]>(), 10);
        assert_eq!(size_of::<&[u16]>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<&str>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<&[u16; 5]>(), size_of::<usize>());
        let a = [1, 2, 3, 4];
        assert_eq!(reversed(a), [4, 3, 2, 1]);
        assert_eq!(a, [1, 2, 3, 4]); // оригинал не изменился
    }

    #[test]
    fn sums_agree() {
        let xs = [1, 2, 3, u32::MAX];
        assert_eq!(ch01_sum_indexed(&xs), 5);
        assert_eq!(ch01_sum_iter(&xs), 5);
        assert_eq!(ch01_sum_first(&xs, 3), 6);
        assert_eq!(ch01_get(&xs, 2), 3);
        assert!(std::panic::catch_unwind(|| ch01_get(&xs, 4)).is_err());
    }

    #[test]
    fn str_is_utf8_bytes() {
        let s = "привет";
        assert_eq!(s.len(), 12);
        assert_eq!(s.chars().count(), 6);
        assert!(!s.is_char_boundary(1));
        assert_eq!(first_char(s), Some("п"));
        assert_eq!(first_char(""), None);
        assert!(std::panic::catch_unwind(|| &s[..1]).is_err());
        assert_eq!(core::str::from_utf8(&[0xD0, 0xBF]), Ok("п"));
        // Первый байт двухбайтовой последовательности без второго.
        let cut = core::hint::black_box([0xD0u8]);
        assert!(core::str::from_utf8(&cut).is_err());
    }
}
