//! Задача C: проба на буфер записи (store buffering). Условие — раздел 21.7.

use std::sync::atomic::Ordering;

// ANCHOR: skeleton
/// Сколько раз из `rounds` наблюдался исход `(0, 0)` в пробе на буфер записи
/// при заданном `order`. Один раунд: две переменные `x`, `y`; поток 1 делает
/// `x.store(1, order); r1 = y.load(order)`, поток 2 — симметрично с `y`/`x`;
/// засчитываем, если `r1 == 0 && r2 == 0`. Потоки запускайте через
/// `thread::scope`, стартуя почти одновременно (одного из двух можно считать
/// текущим потоком).
///
/// При `SeqCst` результат всегда 0; при `Relaxed` — измерьте частоту на своём
/// процессоре.
pub fn count_both_zero(order: Ordering, rounds: usize) -> usize {
    let _ = (order, rounds);
    todo!("раздел 21.7, задача C")
}
// ANCHOR_END: skeleton
