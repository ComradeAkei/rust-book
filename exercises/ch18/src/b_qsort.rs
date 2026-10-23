//! Задача B: отсортируйте срез через `qsort_r` из glibc компаратором-замыканием.
//! Условие — раздел 18.7.

use std::cmp::Ordering;

// ANCHOR: skeleton
/// Объявите `qsort_r` и отсортируйте `data`. Замыкание передаётся как
/// `void *arg`, вызывается переходником `extern "C"` фиксированного типа.
/// Паника в компараторе не должна пересечь границу C — ловите `catch_unwind`.
pub fn sort_by<F: FnMut(&i32, &i32) -> Ordering>(data: &mut [i32], cmp: F) {
    let _ = (data, cmp);
    todo!("раздел 18.7, задача B")
}
// ANCHOR_END: skeleton
