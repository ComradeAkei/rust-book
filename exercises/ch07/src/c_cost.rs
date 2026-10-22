//! Задача C: цена раскрутки в размере бинарника и цена `Result` в горячем
//! цикле.

// ANCHOR: sizes
/// Размеры программы `code/ch07/c-size` после `cargo build --release`,
/// в байтах: файл целиком и четыре секции из `size -A`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sizes {
    pub file: u64,
    pub text: u64,
    pub eh_frame: u64,
    pub eh_frame_hdr: u64,
    pub gcc_except_table: u64,
}

/// Впишите замер с panic = "unwind" (по умолчанию).
pub const UNWIND: Sizes =
    Sizes { file: 0, text: 0, eh_frame: 0, eh_frame_hdr: 0, gcc_except_table: 0 };

/// Впишите замер с panic = "abort".
pub const ABORT: Sizes =
    Sizes { file: 0, text: 0, eh_frame: 0, eh_frame_hdr: 0, gcc_except_table: 0 };
// ANCHOR_END: sizes

// ANCHOR: scale
/// Переполнение при масштабировании.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overflow;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07c_scale(raw: u32, gain: u32) -> Result<u32, Overflow> {
    todo!()
}

/// То же в стиле C: 0 и результат в `*out` или −1.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07c_scale_code(raw: u32, gain: u32, out: &mut u32) -> i32 {
    todo!()
}

/// Сумма `ch07c_scale` по всем отсчётам или первая ошибка.
pub fn sum_result(samples: &[u32], gain: u32) -> Result<u64, Overflow> {
    todo!()
}

/// То же через `ch07c_scale_code`: 0 и сумма в `*out` или −1.
pub fn sum_code(samples: &[u32], gain: u32, out: &mut u64) -> i32 {
    todo!()
}
// ANCHOR_END: scale
