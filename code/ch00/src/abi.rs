//! Раздел 0.4: один исходник, две ISA. Функции с `extern "C"` подчиняются
//! соглашению о вызовах платформы: System V AMD64 на x86_64 Linux и AAPCS на
//! Cortex-M. Ассемблер каждой функции разобран в главе.
//!
//! ```text
//! cargo asm -p ch00 --lib ch00_args8
//! cargo asm -p ch00 --lib --target thumbv7m-none-eabi ch00_args8
//! cargo asm -p ch00 --lib --target thumbv7em-none-eabihf ch00_scale
//! ```

// ANCHOR: args8
/// Восемь аргументов: на x86_64 шесть из них приходят в регистрах, на ARM
/// только четыре. Сдвиги не дают компилятору сократить выражение.
#[inline(never)]
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn ch00_args8(
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    h: u32,
) -> u32 {
    a ^ b.rotate_left(1)
        ^ c.rotate_left(2)
        ^ d.rotate_left(3)
        ^ e.rotate_left(4)
        ^ f.rotate_left(5)
        ^ g.rotate_left(6)
        ^ h.rotate_left(7)
}
// ANCHOR_END: args8

// ANCHOR: mul_wide
/// Результат шире регистра Cortex-M: `u64` возвращается парой r0:r1.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch00_mul_wide(a: u32, b: u32) -> u64 {
    a as u64 * b as u64
}
// ANCHOR_END: mul_wide

// ANCHOR: composite
/// Восемь байт в двух полях. На x86_64 такая структура возвращается в `rax`,
/// а AAPCS возвращает в регистре только составные типы до 4 байт.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pair {
    pub lo: u32,
    pub hi: u32,
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch00_pair(lo: u32, hi: u32) -> Pair {
    Pair { lo, hi }
}

/// 32 байта не помещаются в регистры ни на одной из платформ.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Quad {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: u64,
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch00_quad(x: u64) -> Quad {
    Quad { a: x, b: x + 1, c: x + 2, d: x + 3 }
}
// ANCHOR_END: composite

// ANCHOR: callee_saved
/// Значение `x` нужно и после вызовов `f`, поэтому его приходится держать в
/// регистре, который вызываемая функция обязана сохранить.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch00_keep(x: u32, f: extern "C" fn(u32) -> u32) -> u32 {
    let a = f(x);
    let b = f(a);
    a.wrapping_add(b).wrapping_add(x)
}
// ANCHOR_END: callee_saved

// ANCHOR: float
/// Плавающая точка: в регистрах `xmm` на x86_64, в `s0`, `s1` на Cortex-M4F
/// и в обычных r0, r1 там, где FPU нет.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch00_scale(x: f32, k: f32) -> f32 {
    x * k + 1.0
}
// ANCHOR_END: float

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args8_mixes_every_argument() {
        assert_eq!(ch00_args8(0, 0, 0, 0, 0, 0, 0, 0), 0);
        // Каждый аргумент сдвинут на свою позицию, поэтому единица в любом
        // из них даёт свой бит результата.
        assert_eq!(ch00_args8(1, 1, 1, 1, 1, 1, 1, 1), 0xFF);
        assert_eq!(ch00_args8(0, 0, 0, 0, 0, 0, 0, 1), 1 << 7);
        // Это число печатает прошивка ch00-hello на всех трёх ядрах (раздел 0.6).
        assert_eq!(ch00_args8(1, 2, 3, 4, 5, 6, 7, 8), 0x579);
    }

    #[test]
    fn wide_multiply_does_not_overflow() {
        assert_eq!(ch00_mul_wide(u32::MAX, u32::MAX), 0xFFFF_FFFE_0000_0001);
    }

    #[test]
    fn composites_round_trip() {
        assert_eq!(ch00_pair(1, 2), Pair { lo: 1, hi: 2 });
        assert_eq!(ch00_quad(10), Quad { a: 10, b: 11, c: 12, d: 13 });
    }

    #[test]
    fn keep_calls_back_twice() {
        extern "C" fn double(x: u32) -> u32 {
            x * 2
        }
        // a = 6, b = 12, итог 6 + 12 + 3.
        assert_eq!(ch00_keep(3, double), 21);
    }

    #[test]
    fn scale_is_not_fused() {
        // Rust не сливает умножение и сложение в одну операцию FMA сам,
        // поэтому результат совпадает с двумя отдельными округлениями.
        let (x, k) = (1.1f32, 3.3f32);
        assert_eq!(ch00_scale(x, k), x * k + 1.0);
        assert_eq!(ch00_scale(2.0, 0.5), 2.0);
    }
}
