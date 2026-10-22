//! Задача 4.B: лишняя загрузка в C без `restrict`.
//!
//! Файл собирается и как модуль крейта, и отдельно: тест компилирует его
//! под `thumbv7m-none-eabi` и считает инструкции в `ch04b_add_twice`.
//! Поэтому не добавляйте сюда `use` из других модулей. C-версии пишутся в
//! `c/add_twice.c`.

/// `*a += *b` дважды.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04b_add_twice(a: &mut i32, b: &i32) {
    todo!()
}

/// Сколько инструкций `ldr` в теле на Cortex-M3 (`thumbv7m-none-eabi`,
/// rustc `-O`, clang или GCC `-O2`): Rust, C без `restrict`, C с `restrict`.
pub const LDR: [usize; 3] = [0; 3];

/// То же для `str`.
pub const STR: [usize; 3] = [0; 3];

/// Что окажется в `x`, если передать C-функции один адрес дважды:
/// `int x = 1; add_twice(&x, &x);` и то же для `add_twice_restrict`
/// (компилятор C хоста, `-O2`).
pub const ALIASED: [i32; 2] = [0; 2];
