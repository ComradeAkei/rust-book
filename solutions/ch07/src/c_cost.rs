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

/// rustc 1.97.0, x86_64-unknown-linux-gnu, panic = "unwind".
pub const UNWIND: Sizes = Sizes {
    file: 462_096,
    text: 260_611,
    eh_frame: 22_008,
    eh_frame_hdr: 4_412,
    gcc_except_table: 11_140,
};

/// То же с `--config 'profile.release.panic="abort"'`.
pub const ABORT: Sizes = Sizes {
    file: 458_880,
    text: 259_059,
    eh_frame: 21_684,
    eh_frame_hdr: 4_364,
    gcc_except_table: 10_984,
};
// ANCHOR_END: sizes

// ANCHOR: scale
/// Переполнение при масштабировании.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overflow;

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07c_scale(raw: u32, gain: u32) -> Result<u32, Overflow> {
    raw.checked_mul(gain).ok_or(Overflow)
}

/// То же в стиле C: 0 и результат в `*out` или −1.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07c_scale_code(raw: u32, gain: u32, out: &mut u32) -> i32 {
    match raw.checked_mul(gain) {
        Some(v) => {
            *out = v;
            0
        }
        None => -1,
    }
}

pub fn sum_result(samples: &[u32], gain: u32) -> Result<u64, Overflow> {
    let mut sum = 0u64;
    for &s in samples {
        sum += ch07c_scale(s, gain)? as u64;
    }
    Ok(sum)
}

pub fn sum_code(samples: &[u32], gain: u32, out: &mut u64) -> i32 {
    let mut sum = 0u64;
    for &s in samples {
        let mut v = 0;
        if ch07c_scale_code(s, gain, &mut v) != 0 {
            return -1;
        }
        sum += v as u64;
    }
    *out = sum;
    0
}
// ANCHOR_END: scale
