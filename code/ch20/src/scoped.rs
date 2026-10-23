//! Раздел 20.4: области потоков `thread::scope`. `thread::spawn` требует
//! `'static`: поток может пережить вызвавший кадр, поэтому одалживать его
//! локальные данные нельзя — только переносить (`move`) или делить через
//! `Arc`. Но для временного параллелизма — «посчитать на N потоках и
//! дождаться всех здесь же» — это лишняя аллокация. `thread::scope` даёт
//! потоки, которые гарантированно завершатся до конца области, поэтому им
//! можно одалживать данные из стека по `&`.

use std::thread;

// ANCHOR: scope
/// Параллельная сумма среза на `chunks` потоках без `Arc`: каждый поток
/// берёт `&`-ссылку на свой кусок исходного среза. `scope` дожидается всех
/// потоков перед возвратом, поэтому компилятор разрешает одолжить `data` —
/// он знает, что ни один поток не переживёт этот вызов.
pub fn parallel_sum(data: &[u64], chunks: usize) -> u64 {
    let size = data.len().div_ceil(chunks.max(1));
    thread::scope(|s| {
        let handles: Vec<_> = data
            .chunks(size.max(1))
            .map(|chunk| s.spawn(move || chunk.iter().sum::<u64>()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    })
}
// ANCHOR_END: scope

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_sum_matches_sequential() {
        let data: Vec<u64> = (1..=1000).collect();
        let expected: u64 = data.iter().sum();
        for chunks in [1, 3, 4, 7, 16] {
            assert_eq!(parallel_sum(&data, chunks), expected, "chunks={chunks}");
        }
    }

    #[test]
    fn parallel_sum_handles_empty() {
        assert_eq!(parallel_sum(&[], 4), 0);
    }
}
