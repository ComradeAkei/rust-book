//! Раздел A.2: целые числа и арифметика.
//!
//! Часть тестов ведёт себя по-разному в debug и release: проверки
//! переполнения по умолчанию включены вместе с `debug_assertions`. Поэтому
//! крейт стоит прогонять в обоих профилях: `cargo test` и
//! `cargo test --release`.

// ANCHOR: square
/// Перенос `square` из `c/promote.c`. Расширение до `u32` записано явно,
/// и произведение двух `u16` в `u32` переполниться не может.
pub fn square(x: u16) -> u32 {
    let x = u32::from(x);
    x * x
}

/// Буквальный перенос: умножение идёт в `u16` и переполняется.
pub fn square_naive(x: u16) -> u32 {
    (x * x) as u32
}
// ANCHOR_END: square

// ANCHOR: quot
/// Деление `i32`. Проверку `i32::MIN / -1` компилятор вставляет всегда,
/// в том числе в release.
pub fn quot(a: i32, b: i32) -> i32 {
    a / b
}
// ANCHOR_END: quot

/// Сдвиг влево на произвольное число бит.
pub fn shl(x: u32, n: u32) -> u32 {
    x << n
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hint::black_box;
    use std::panic::catch_unwind;

    /// Текст паники или `None`, если функция завершилась без неё.
    fn panic_message<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> Option<String> {
        let err = catch_unwind(f).err()?;
        err.downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| err.downcast_ref::<String>().cloned())
    }

    #[test]
    fn promotion_must_be_explicit() {
        assert_eq!(square(65535), 4_294_836_225);
        let naive = panic_message(|| square_naive(black_box(65535)));
        if cfg!(debug_assertions) {
            assert_eq!(naive.as_deref(), Some("attempt to multiply with overflow"));
        } else {
            // В release умножение u16 идёт по модулю 2^16: 0xFFFF² = 0xFFFE_0001.
            assert_eq!(naive, None);
            assert_eq!(square_naive(black_box(65535)), 1);
        }
    }

    // ANCHOR: division_tests
    #[test]
    fn division_overflow_panics_in_every_profile() {
        let msg = panic_message(|| quot(black_box(i32::MIN), black_box(-1)));
        assert_eq!(msg.as_deref(), Some("attempt to divide with overflow"));
        let msg = panic_message(|| quot(black_box(1), black_box(0)));
        assert_eq!(msg.as_deref(), Some("attempt to divide by zero"));
        assert_eq!(i32::MIN.checked_div(-1), None);
        assert_eq!(i32::MIN.wrapping_div(-1), i32::MIN);
    }
    // ANCHOR_END: division_tests

    #[test]
    fn shift_by_width() {
        let msg = panic_message(|| shl(black_box(1), black_box(32)));
        if cfg!(debug_assertions) {
            assert_eq!(msg.as_deref(), Some("attempt to shift left with overflow"));
        } else {
            // В release число бит берётся по модулю ширины: 32 & 31 = 0.
            assert_eq!(msg, None);
            assert_eq!(shl(black_box(1), black_box(32)), 1);
        }
        assert_eq!(1u32.checked_shl(32), None);
        assert_eq!(1u32.wrapping_shl(32), 1);
        // Сдвиг вправо знакового числа арифметический (в C до C23 это
        // поведение определялось реализацией).
        assert_eq!(-16i32 >> 2, -4);
    }

    // ANCHOR: overflow_api
    #[test]
    fn overflow_api() {
        let x: u8 = 250;
        assert_eq!(x.checked_add(10), None);
        assert_eq!(x.wrapping_add(10), 4);
        assert_eq!(x.saturating_add(10), 255);
        assert_eq!(x.overflowing_add(10), (4, true));
        assert_eq!(i8::MIN.checked_abs(), None);
        assert_eq!(i8::MIN.unsigned_abs(), 128);
    }
    // ANCHOR_END: overflow_api

    // ANCHOR: casts
    #[test]
    #[allow(clippy::cast_nan_to_int)] // приведение NaN здесь и проверяется
    fn as_casts() {
        assert_eq!(300i32 as u8, 44); // отбрасываются старшие биты
        assert_eq!(-1i32 as u32, u32::MAX); // биты те же
        assert_eq!(-1i8 as i32, -1); // знаковое расширение
        assert_eq!(0xFFu8 as i8 as i32, -1);
        assert_eq!(u8::try_from(300i32).ok(), None); // проверенное сужение
        // Вещественное в целое насыщается, NaN даёт 0. В C это UB.
        assert_eq!(3e9f64 as i32, i32::MAX);
        assert_eq!(-1e10f64 as i32, i32::MIN);
        assert_eq!(f64::NAN as i32, 0);
        assert_eq!(-1.5f64 as u8, 0);
        assert_eq!(2.9f64 as u8, 2); // отбрасывание дробной части, как в C
    }
    // ANCHOR_END: casts

    #[test]
    fn remainder() {
        // `%` усекает к нулю, как в C99.
        assert_eq!(-7 % 3, -1);
        assert_eq!(7 % -3, 1);
        // Математический остаток всегда неотрицателен.
        assert_eq!((-7i32).rem_euclid(3), 2);
        assert_eq!((-7i32).div_euclid(3), -3);
    }

    #[test]
    fn unchecked_is_c_semantics() {
        // SAFETY: 2 + 3 не переполняет i32. При переполнении это было бы
        // UB, как сложение int в C: ровно это и обещает unchecked_add.
        let s = unsafe { 2i32.unchecked_add(3) };
        assert_eq!(s, 5);
    }
}
