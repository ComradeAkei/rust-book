//! Раздел A.8: препроцессор, атрибуты и встроенные функции компилятора.

// ANCHOR: max_macro
/// Перенос `#define MAX(a, b) ((a) > (b) ? (a) : (b))` «как есть».
/// Гигиена macro_rules защищает имена, но не число вычислений:
/// `$a` подставлен дважды и дважды выполнится.
#[macro_export]
macro_rules! max_twice {
    ($a:expr, $b:expr) => {
        if $a > $b { $a } else { $b }
    };
}

/// Каждый аргумент вычисляется ровно один раз, как у функции.
#[macro_export]
macro_rules! max_once {
    ($a:expr, $b:expr) => {
        match ($a, $b) {
            (a, b) => {
                if a > b {
                    a
                } else {
                    b
                }
            }
        }
    };
}
// ANCHOR_END: max_macro

// ANCHOR: build_time
/// `_Static_assert(sizeof(void *) == 8 || sizeof(void *) == 4, "...")`.
const _: () = assert!(size_of::<usize>() == 8 || size_of::<usize>() == 4);

/// `#if defined(__ARM_ARCH_6M__)` и `#ifdef` вообще: выбор при компиляции.
pub const fn has_cas() -> bool {
    cfg!(target_has_atomic = "32")
}
// ANCHOR_END: build_time

#[cfg(test)]
mod tests {
    #[test]
    fn macro_double_evaluation() {
        // C: int i = 5, j = 3; m = MAX(i++, j); даёт m = 6, i = 7.
        let mut i = 5;
        let mut post_inc = || {
            let v = i;
            i += 1;
            v
        };
        let m = max_twice!(post_inc(), 3);
        assert_eq!((m, i), (6, 7));

        let mut i = 5;
        let mut post_inc = || {
            let v = i;
            i += 1;
            v
        };
        let m = max_once!(post_inc(), 3);
        assert_eq!((m, i), (5, 6));
        assert_eq!(core::cmp::max(5, 3), 5);
    }

    #[test]
    fn host_has_cas() {
        assert!(super::has_cas()); // x86_64; на thumbv6m было бы false
    }

    // ANCHOR: builtins
    #[test]
    fn builtins() {
        // __builtin_clz(0) и __builtin_ctz(0) в C не определены.
        assert_eq!(0u32.leading_zeros(), 32);
        assert_eq!(0u32.trailing_zeros(), 32);
        assert_eq!(1u32.leading_zeros(), 31);
        assert_eq!(0xF0u32.count_ones(), 4); // __builtin_popcount
        assert_eq!(0x1234_5678u32.swap_bytes(), 0x7856_3412); // __builtin_bswap32
        assert_eq!(0x8000_0001u32.rotate_left(1), 3); // в C только идиома сдвигов
        assert_eq!(0x1234u16.to_be_bytes(), [0x12, 0x34]); // htons и запись в буфер
        assert_eq!(u16::from_be_bytes([0x12, 0x34]), 0x1234); // ntohs
        assert_eq!(250u8.checked_add(10), None); // __builtin_add_overflow
    }
    // ANCHOR_END: builtins

    #[test]
    fn source_location() {
        // __FILE__, __LINE__ и __func__.
        assert!(file!().ends_with("preproc.rs"));
        assert_eq!(module_path!(), "appendix_a::preproc::tests");
    }
}
