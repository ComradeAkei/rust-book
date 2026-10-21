//! Задача 0.C. Сумма массива `u32` на x86_64 и Cortex-M4F.
//!
//! Реализуйте три варианта суммы. Атрибуты уже стоят: `no_mangle` делает
//! имена одинаковыми под любой целью, `inline(never)` не даёт функциям
//! раствориться в тестах.
//!
//! ```text
//! cargo test -p ch00-exercises task_c
//! cargo asm -p ch00-exercises --lib ch00c_sum_wrapping
//! cargo asm -p ch00-exercises --lib --target thumbv7em-none-eabihf ch00c_sum_wrapping
//! ```
//!
//! Отчёт: для каждой функции и каждой цели объясните, в каких регистрах
//! пришли указатель и длина, во что превратился цикл (векторизация, развёртка,
//! хвост) и почему варианты различаются между собой.

/// Сумма по модулю 2³², как сложение `u32` в C.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00c_sum_wrapping(xs: &[u32]) -> u32 {
    todo!()
}

/// Точная сумма в `u64`: переполнения нет ни на каком реальном срезе.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00c_sum_wide(xs: &[u32]) -> u64 {
    todo!()
}

/// Точная сумма, если она помещается в `u32`, иначе `None`.
///
/// Вторая часть задачи: сделайте так, чтобы на x86_64 этот вариант был
/// векторизован, как `ch00c_sum_wide`, и докажите это ассемблером.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00c_sum_checked(xs: &[u32]) -> Option<u32> {
    todo!()
}
