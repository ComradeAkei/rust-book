//! Раздел B.1: `unsafe extern` и небезопасные атрибуты.

use core::ffi::{c_char, c_int};

// ANCHOR: unsafe_extern
// Блок `extern` сам по себе обещание: сигнатуры ниже совпадают с теми, что
// в libc. Компилятор проверить это не может, поэтому в 2024 блок пишется
// как `unsafe extern`, а для каждой функции отдельно решается, безопасен
// ли её вызов.
unsafe extern "C" {
    /// Аргументов нет, состояние не меняется: вызов безопасен всегда.
    pub safe fn getpid() -> c_int;

    /// Читает память до NUL: за указатель отвечает вызывающий.
    pub unsafe fn strlen(s: *const c_char) -> usize;
}
// ANCHOR_END: unsafe_extern

// ANCHOR: unsafe_attrs
/// Символ с именем `appb_tick` без декорирования, для кода на C или
/// ассемблере. Атрибут небезопасен: функция с именем `malloc` или `memcpy`
/// молча подменила бы функцию libc во всей программе.
#[unsafe(no_mangle)]
pub extern "C" fn appb_tick(counter: u32) -> u32 {
    counter.wrapping_add(1)
}

/// Таблица в своей секции, как `__attribute__((section(...), used))`.
/// Небезопасно по той же причине: у секций вроде `.init_array` особый
/// смысл, и данные, положенные туда по ошибке, выполнятся как код.
#[unsafe(link_section = ".rodata.appb_table")]
#[used]
pub static TABLE: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
// ANCHOR_END: unsafe_attrs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "вызов функции libc")]
    fn safe_foreign_fn() {
        // Без блока unsafe: функция объявлена как `safe fn`.
        assert_eq!(getpid() as u32, std::process::id());
    }

    #[test]
    fn unsafe_foreign_fn() {
        let s = c"telemetry";
        // SAFETY: `s` — корректная C-строка с NUL на конце, живёт до конца теста.
        let n = unsafe { strlen(s.as_ptr()) };
        assert_eq!(n, 9);
    }

    #[test]
    fn exported_items() {
        assert_eq!(appb_tick(u32::MAX), 0);
        assert_eq!(TABLE, [0xDE, 0xAD, 0xBE, 0xEF]);
    }
}
