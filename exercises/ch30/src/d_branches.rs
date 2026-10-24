//! Задача D: ветвление против кода без ветвлений.

// ANCHOR: search
/// Первый индекс `i`, где `v[i] >= x`, в отсортированном `v` — то же, что
/// `v.partition_point(|&e| e < x)`. Переход по результату сравнения.
#[inline(never)]
pub fn lower_bound_branchy(v: &[u32], x: u32) -> usize {
    let _ = (v, x);
    todo!("30.D1: поиск с ветвлением")
}

/// То же без условного перехода по данным.
#[inline(never)]
pub fn lower_bound_branchless(v: &[u32], x: u32) -> usize {
    let _ = (v, x);
    todo!("30.D1: поиск без ветвления")
}
// ANCHOR_END: search

// ANCHOR: codegen
/// Что LLVM сделал из `if`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codegen {
    /// Условный переход.
    Branch,
    /// Без условного перехода: `cmov`, `set<cc>`, арифметика.
    Branchless,
}

#[inline(never)]
pub fn max_of(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

#[inline(never)]
pub fn first_or_zero(v: &[u32]) -> u32 {
    if v.is_empty() { 0 } else { v[0] }
}

// Предмет задачи — `if` в том виде, как его пишут на C, а не checked_div.
#[allow(clippy::manual_checked_ops)]
#[inline(never)]
pub fn div_or_zero(a: u32, d: u32) -> u32 {
    if d == 0 { 0 } else { a / d }
}

#[inline(never)]
fn log_value(x: u32) -> u32 {
    std::hint::black_box(x)
}

#[inline(never)]
pub fn call_if(c: bool, x: u32) -> u32 {
    if c { log_value(x) } else { x }
}

#[inline(never)]
pub fn clamp_byte(x: i32) -> u8 {
    if x < 0 {
        0
    } else if x > 255 {
        255
    } else {
        x as u8
    }
}

#[inline(never)]
pub fn heavy_if(c: bool, x: u64) -> u64 {
    if c { x.wrapping_mul(x).wrapping_mul(x).wrapping_mul(x).wrapping_mul(x) } else { x }
}

#[inline(never)]
pub fn mul_if(c: bool, x: u64) -> u64 {
    if c { x.wrapping_mul(3) } else { x }
}

#[inline(never)]
pub fn nonzero_and_even(a: &u32, b: &u32) -> bool {
    *a != 0 && b.is_multiple_of(2)
}

#[inline(never)]
pub fn get_or(v: &[u32], i: usize, d: u32) -> u32 {
    if i < v.len() { v[i] } else { d }
}
// ANCHOR_END: codegen

// ANCHOR: answers
/// Что LLVM сделал из `if` в функциях [`max_of`] … [`get_or`], по порядку.
pub fn predictions() -> [Codegen; 9] {
    todo!("30.D2: предскажите, потом проверьте по ассемблеру")
}
// ANCHOR_END: answers
