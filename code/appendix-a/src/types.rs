//! Раздел A.1: типы данных.

// ANCHOR: array_by_value
/// Массив передаётся по значению целиком и не распадается в указатель,
/// поэтому функция меняет свою копию, а не массив вызывающего.
pub fn zero_first(mut a: [u8; 4]) -> [u8; 4] {
    a[0] = 0;
    a
}
// ANCHOR_END: array_by_value

// ANCHOR: c_enum
/// `enum mode { MODE_IDLE, MODE_RUN, MODE_SLEEP }` из заголовка C в том
/// виде, в каком он пересекает границу FFI: просто число. C-код вправе
/// записать в такую переменную любое значение, и Rust должен это пережить.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RawMode(pub u32);

/// Тот же режим внутри Rust-кода. Значение вне списка здесь невозможно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Run,
    Sleep,
}

impl TryFrom<RawMode> for Mode {
    type Error = RawMode;

    fn try_from(raw: RawMode) -> Result<Self, RawMode> {
        match raw.0 {
            0 => Ok(Mode::Idle),
            1 => Ok(Mode::Run),
            2 => Ok(Mode::Sleep),
            _ => Err(raw),
        }
    }
}
// ANCHOR_END: c_enum

// ANCHOR: c_sizes
/// Перечисление с раскладкой C. Его размер выбирает ABI цели, а не Rust.
#[repr(C)]
pub enum CEnum {
    A,
    B,
    C,
}

const _: () = {
    use core::ffi::{c_char, c_int, c_long};
    use core::mem::size_of;
    assert!(size_of::<c_int>() == 4);
    #[cfg(target_arch = "x86_64")]
    {
        assert!(size_of::<c_long>() == 8); // LP64
        assert!(c_char::MIN == -128); // char знаковый
        assert!(size_of::<CEnum>() == 4); // перечисление размером с int
    }
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        assert!(size_of::<c_long>() == 4); // ILP32
        assert!(c_char::MIN == 0); // char беззнаковый, так требует AAPCS
        assert!(size_of::<CEnum>() == 1); // короткие перечисления ARM EABI
    }
};
// ANCHOR_END: c_sizes

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;
    use core::ptr::NonNull;

    #[test]
    fn array_is_copied() {
        let a = [1, 2, 3, 4];
        let b = zero_first(a);
        assert_eq!(a, [1, 2, 3, 4]);
        assert_eq!(b, [0, 2, 3, 4]);
    }

    #[test]
    fn sizes() {
        let word = size_of::<usize>();
        assert_eq!(size_of::<char>(), 4);
        assert_eq!(size_of::<bool>(), 1);
        assert_eq!(size_of::<()>(), 0);
        assert_eq!(size_of::<*const u8>(), word);
        // Указатель и длина: «толстый» указатель из двух слов.
        assert_eq!(size_of::<&[u8]>(), 2 * word);
        assert_eq!(size_of::<&str>(), 2 * word);
        // NULL кодируется нулём адреса, отдельного флага нет.
        assert_eq!(size_of::<Option<&u8>>(), word);
        assert_eq!(size_of::<Option<NonNull<u8>>>(), word);
        assert_eq!(size_of::<Option<fn()>>(), word);
    }

    #[test]
    fn foreign_mode_is_checked() {
        assert_eq!(Mode::try_from(RawMode(1)), Ok(Mode::Run));
        assert_eq!(Mode::try_from(RawMode(7)), Err(RawMode(7)));
    }

    // ANCHOR: ub_enum
    /// Так делать нельзя: значение 7 недопустимо для `Mode`, и уже само
    /// создание такого значения — неопределённое поведение. Тест помечен
    /// `#[ignore]` и нужен только для запуска под Miri:
    /// `cargo +nightly miri test -p appendix-a -- --ignored ub_enum`.
    #[test]
    #[ignore = "UB: запускать только под Miri"]
    fn ub_enum_from_c() {
        let raw: u8 = 7; // пришло из C
        let mode: Mode = unsafe { core::mem::transmute(raw) };
        assert_ne!(mode, Mode::Idle);
    }
    // ANCHOR_END: ub_enum
}
