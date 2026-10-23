//! Задача B: вызвать `qsort_r` из glibc с компаратором-замыканием. Замыкание
//! передаётся как `void *arg`, а вызывается переходником фиксированного типа.
//! Паника внутри компаратора не должна пересечь границу C (это UB), поэтому
//! её ловит `catch_unwind` и возвращает «равно».

use core::cmp::Ordering;
use core::ffi::{c_int, c_void};
use std::panic::{self, AssertUnwindSafe};

unsafe extern "C" {
    // void qsort_r(void *base, size_t nmemb, size_t size,
    //              int (*compar)(const void *, const void *, void *),
    //              void *arg);
    fn qsort_r(
        base: *mut c_void,
        nmemb: usize,
        size: usize,
        compar: extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int,
        arg: *mut c_void,
    );
}

// ANCHOR: sort
/// Отсортировать срез `i32` компаратором-замыканием через `qsort_r`.
pub fn sort_by<F: FnMut(&i32, &i32) -> Ordering>(data: &mut [i32], mut cmp: F) {
    extern "C" fn trampoline<F: FnMut(&i32, &i32) -> Ordering>(
        a: *const c_void,
        b: *const c_void,
        arg: *mut c_void,
    ) -> c_int {
        // SAFETY: a и b — указатели на элементы массива i32 (выровнены, живы);
        // arg — &mut F, переданный в sort_by ниже.
        let (a, b) = unsafe { (&*a.cast::<i32>(), &*b.cast::<i32>()) };
        let f = unsafe { &mut *arg.cast::<F>() };
        // Ordering как i32 даёт -1/0/1 — ровно знак, которого ждёт qsort.
        let ord = panic::catch_unwind(AssertUnwindSafe(|| f(a, b))).unwrap_or(Ordering::Equal);
        ord as c_int
    }

    let arg = (&mut cmp as *mut F).cast::<c_void>();
    // SAFETY: base/nmemb/size верно описывают срез data; trampoline::<F>
    // имеет требуемый тип; arg указывает на cmp, живую весь вызов qsort_r.
    unsafe {
        qsort_r(data.as_mut_ptr().cast(), data.len(), size_of::<i32>(), trampoline::<F>, arg);
    }
}
// ANCHOR_END: sort
