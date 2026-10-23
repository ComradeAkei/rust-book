//! Задача B: `qsort` из libc против `sort_unstable_by` на одних данных.
//! Время меряет пример `examples/b_bench.rs`, тесты проверяют порядок и
//! то, что компаратор Rust встроился, а компаратор C — нет.

use core::cmp::Ordering;
use core::ffi::{c_int, c_void};

// ANCHOR: ffi
unsafe extern "C" {
    /// `qsort` из стандартной библиотеки C (ISO C11, 7.22.5.2).
    fn qsort(
        base: *mut c_void,
        nmemb: usize,
        size: usize,
        compar: unsafe extern "C" fn(*const c_void, *const c_void) -> c_int,
    );
}
// ANCHOR_END: ffi

// ANCHOR: api
/// Компаратор для `qsort`: отрицательное, ноль или положительное.
///
/// # Safety
///
/// `a` и `b` указывают на выровненные `u32`.
pub unsafe extern "C" fn cmp_u32(a: *const c_void, b: *const c_void) -> c_int {
    // Не todo!(): паника не может выйти из extern "C" и обрывает весь
    // процесс тестов. Заглушка считает все числа равными.
    0
}

/// Отсортировать через `qsort` с компаратором `cmp_u32`.
pub fn sort_libc(v: &mut [u32]) {
    todo!()
}

/// Отсортировать через `sort_unstable_by` с замыканием.
pub fn sort_rust(v: &mut [u32]) {
    todo!()
}
// ANCHOR_END: api

/// Порядок, который задаёт `cmp_u32`, — для проверок.
pub fn order(a: u32, b: u32) -> Ordering {
    // SAFETY: указатели на локальные u32.
    let r = unsafe { cmp_u32((&raw const a).cast(), (&raw const b).cast()) };
    r.cmp(&0)
}
