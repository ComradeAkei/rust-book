//! Раздел 0.5: две ошибки, которые проходят компиляцию и обычный запуск.
//! Их находят Miri и AddressSanitizer.
//!
//! ```text
//! cargo run -p ch00 --example ub                          # печатает что-то правдоподобное
//! cargo +nightly miri run -p ch00 --example ub            # Miri: первая ошибка
//! cargo +nightly miri run -p ch00 --example ub -- average # Miri: вторая ошибка
//! RUSTFLAGS=-Zsanitizer=address cargo +nightly run -p ch00 --example ub \
//!     --target x86_64-unknown-linux-gnu                   # ASan: heap-use-after-free
//! ```
//!
//! Код намеренно неверен. Не повторяйте его за пределами этого примера.

// ANCHOR: stale
/// Указатель на первый элемент, сохранённый до `push`. Если `push`
/// перевыделил буфер, указатель смотрит в освобождённую память.
fn first_after_push(v: &mut Vec<u32>) -> u32 {
    let first: *const u32 = v.as_ptr();
    v.push(4); // ёмкость 3 исчерпана: realloc
    // SAFETY: НАРУШЕНО. После перевыделения `first` висячий.
    unsafe { *first }
}
// ANCHOR_END: stale

// ANCHOR: off_by_one
/// Среднее по срезу с ошибкой на единицу: читает элемент за концом.
fn average(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for i in 1..=xs.len() {
        // SAFETY: НАРУШЕНО. При i == xs.len() индекс за границей.
        sum += unsafe { *xs.get_unchecked(i) };
    }
    sum / xs.len() as u32
}
// ANCHOR_END: off_by_one

/// Без аргументов запускает обе ошибки по очереди. Miri останавливается на
/// первой, поэтому вторую можно запустить отдельно: `-- average`.
fn main() {
    if std::env::args().nth(1).as_deref() != Some("average") {
        let mut v = Vec::with_capacity(3);
        v.extend_from_slice(&[1, 2, 3]);
        println!("первый элемент: {}", first_after_push(&mut v));
    }
    println!("среднее: {}", average(&[10, 20, 30]));
}
