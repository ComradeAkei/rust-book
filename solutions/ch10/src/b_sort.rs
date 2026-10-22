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

// ANCHOR: cmp
/// Компаратор для `qsort`: отрицательное, ноль или положительное.
/// Разность `a - b` здесь не годится: для `u32` она не помещается в
/// `int`, и порядок ломается на больших числах.
///
/// # Safety
///
/// `a` и `b` указывают на выровненные `u32`.
pub unsafe extern "C" fn cmp_u32(a: *const c_void, b: *const c_void) -> c_int {
    // SAFETY: по контракту функции; qsort передаёт указатели на элементы
    // массива, который ей дал sort_libc, — это u32.
    let (a, b) = unsafe { (*a.cast::<u32>(), *b.cast::<u32>()) };
    a.cmp(&b) as c_int
}

/// Отсортировать через `qsort`: одна функция в libc на все типы, сравнение
/// — косвенный вызов `cmp_u32`.
pub fn sort_libc(v: &mut [u32]) {
    // Пустой срез даёт висячий указатель, а C требует и при nmemb = 0
    // «допустимое значение» указателя. Сортировать нечего — не зовём.
    if v.len() < 2 {
        return;
    }
    // SAFETY: base и nmemb описывают живой изменяемый срез, size — размер
    // элемента, cmp_u32 читает только u32 и не паникует.
    unsafe { qsort(v.as_mut_ptr().cast(), v.len(), size_of::<u32>(), cmp_u32) }
}
// ANCHOR_END: cmp

// ANCHOR: rust
/// Отсортировать через `sort_unstable_by`: сортировка мономорфизуется для
/// `u32` и этого замыкания, и сравнение встраивается в её тело.
// clippy предлагает sort_unstable: код тот же, но задача сравнивает
// именно сортировку с замыканием-компаратором.
#[allow(clippy::unnecessary_sort_by)]
pub fn sort_rust(v: &mut [u32]) {
    v.sort_unstable_by(|a, b| a.cmp(b));
}
// ANCHOR_END: rust

/// Порядок, который задаёт `cmp_u32`, — для проверок.
pub fn order(a: u32, b: u32) -> Ordering {
    // SAFETY: указатели на локальные u32.
    let r = unsafe { cmp_u32((&raw const a).cast(), (&raw const b).cast()) };
    r.cmp(&0)
}
