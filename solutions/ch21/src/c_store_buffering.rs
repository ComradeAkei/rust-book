//! Задача C: проба на буфер записи (store buffering). Два потока, две
//! переменные. Поток 1: `x = 1; r1 = y`. Поток 2: `y = 1; r2 = x`. Если оба
//! прочитали ноль, значит, каждый не увидел чужую запись — она «застряла» в
//! буфере записи процессора. При Relaxed (и даже при Release/Acquire!) исход
//! `(0, 0)` допустим моделью и встречается на железе; при SeqCst есть единый
//! глобальный порядок, и такого исхода нет.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

// ANCHOR: solution
/// Один раунд пробы при заданном `Ordering` для store и load. Возвращает
/// `true`, если наблюдался исход `(0, 0)`. Потоки стартуют почти
/// одновременно, чтобы поймать окно буфера записи.
fn one_round(order: Ordering) -> bool {
    let x = AtomicUsize::new(0);
    let y = AtomicUsize::new(0);
    thread::scope(|s| {
        let t1 = s.spawn(|| {
            x.store(1, order);
            y.load(order)
        });
        // Второй «поток» — текущий: x.store уже в t1, здесь симметрично.
        y.store(1, order);
        let r2 = x.load(order);
        let r1 = t1.join().unwrap();
        r1 == 0 && r2 == 0
    })
}

/// Сколько раз из `rounds` наблюдался исход `(0, 0)`. При `Ordering::SeqCst`
/// это всегда 0; при `Ordering::Relaxed` — обычно больше нуля на реальном
/// x86 (частота зависит от процессора и планировщика).
pub fn count_both_zero(order: Ordering, rounds: usize) -> usize {
    (0..rounds).filter(|_| one_round(order)).count()
}
// ANCHOR_END: solution

#[cfg(test)]
mod extra {
    // Проверка в tests-shared/ch21.rs: SeqCst не даёт (0,0); Relaxed —
    // наблюдение студента (на разном железе частота разная), поэтому жёстко
    // не проверяется.
}
