//! Раздел 1.2: переполнение и семейства методов для него.
//!
//! ```text
//! cargo asm -p ch01 --lib --config profile.release.overflow-checks=true ch01_plus
//! cargo asm -p ch01 --lib ch01_checked_add_i32           # флаг OF на x86_64
//! cargo asm -p ch01 --lib --target thumbv7m-none-eabi ch01_checked_add_i32
//! ```

// ANCHOR: plain
/// Обычное сложение. Поведение при переполнении задаёт профиль: паника при
/// `overflow-checks = true`, заворачивание без них.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_plus(a: u32, b: u32) -> u32 {
    a + b
}
// ANCHOR_END: plain

// ANCHOR: families
/// Заворачивание по модулю 2³² в любом профиле.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_wrapping_add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

/// `None` при переполнении. Для беззнаковых это флаг переноса (CF, C).
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_checked_add_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

/// Для знаковых — флаг переполнения (OF на x86, V на ARM).
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_checked_add_i32(a: i32, b: i32) -> Option<i32> {
    a.checked_add(b)
}

/// Результат по модулю и признак переполнения.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_overflowing_add_i32(a: i32, b: i32) -> (i32, bool) {
    a.overflowing_add(b)
}

/// Насыщение: при переполнении результат прижимается к границе типа.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_saturating_add_u32(a: u32, b: u32) -> u32 {
    a.saturating_add(b)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_saturating_add_i32(a: i32, b: i32) -> i32 {
    a.saturating_add(b)
}

/// Паника при переполнении в любом профиле, в том числе в release.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_strict_add(a: u32, b: u32) -> u32 {
    a.strict_add(b)
}
// ANCHOR_END: families

// ANCHOR: div
/// Деление проверяется в любом профиле: на ноль и `i32::MIN / -1`. Второе
/// — тоже переполнение: результат 2³¹ не помещается в `i32`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_div(a: i32, b: i32) -> i32 {
    a / b
}
// ANCHOR_END: div

// ANCHOR: shl
/// Сдвиг на ширину типа и больше — тоже переполнение. Без проверок
/// величина сдвига берётся по модулю 32, как у `wrapping_shl`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_shl(x: u32, n: u32) -> u32 {
    x << n
}
// ANCHOR_END: shl

#[cfg(test)]
mod tests {
    use super::*;
    use core::hint::black_box;

    #[test]
    fn families() {
        assert_eq!(ch01_wrapping_add(i32::MAX, 1), i32::MIN);
        assert_eq!(ch01_checked_add_u32(u32::MAX, 1), None);
        assert_eq!(ch01_checked_add_u32(1, 2), Some(3));
        assert_eq!(ch01_checked_add_i32(i32::MAX, 1), None);
        assert_eq!(ch01_checked_add_i32(-1, i32::MIN + 1), Some(i32::MIN));
        assert_eq!(ch01_overflowing_add_i32(i32::MAX, 1), (i32::MIN, true));
        assert_eq!(ch01_saturating_add_u32(u32::MAX - 1, 5), u32::MAX);
        assert_eq!(ch01_saturating_add_i32(i32::MIN, -5), i32::MIN);
    }

    // Беззнаковое переполнение и знаковое — разные события: 0x7FFF_FFFF + 1
    // переполняет i32, но не u32, а 0xFFFF_FFFF + 1 наоборот.
    #[test]
    fn carry_and_overflow_differ() {
        assert_eq!(0x7FFF_FFFFu32.overflowing_add(1), (0x8000_0000, false));
        assert_eq!(0x7FFF_FFFFi32.overflowing_add(1), (i32::MIN, true));
        assert_eq!(0xFFFF_FFFFu32.overflowing_add(1), (0, true));
        assert_eq!((-1i32).overflowing_add(1), (0, false));
    }

    #[test]
    fn strict_panics_in_every_profile() {
        let r = std::panic::catch_unwind(|| ch01_strict_add(black_box(u32::MAX), 1));
        assert!(r.is_err());
    }

    #[test]
    fn division_is_checked_in_every_profile() {
        assert!(std::panic::catch_unwind(|| ch01_div(black_box(i32::MIN), -1)).is_err());
        assert!(std::panic::catch_unwind(|| ch01_div(black_box(1), 0)).is_err());
        assert_eq!(i32::MIN.wrapping_div(-1), i32::MIN);
        assert_eq!(i32::MIN.checked_div(-1), None);
    }

    #[test]
    fn shift_follows_profile() {
        let r = std::panic::catch_unwind(|| ch01_shl(1, black_box(33)));
        if cfg!(debug_assertions) {
            assert!(r.is_err());
        } else {
            assert_eq!(r.unwrap(), 2);
        }
        assert_eq!(1u32.wrapping_shl(33), 2);
        assert_eq!(1u32.checked_shl(32), None);
    }
}
