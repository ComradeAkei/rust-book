//! Раздел 14.5: запись через указатель, полученный из общей ссылки, —
//! неопределённое поведение. Отладочная сборка печатает 100, release — 2:
//! оптимизатор верит, что за `&u32` значение во время вызова не меняется.
//!
//! Запуск: `cargo run -p ch14 --example sneaky` и то же с `--release`;
//! `cargo +nightly miri run -p ch14 --example sneaky` находит ошибку.

#[inline(never)]
fn sneaky(p: *const u32) {
    // SAFETY: никакой: запись в значение за общей ссылкой без UnsafeCell —
    // ровно та ошибка, которую показывает пример.
    unsafe { *(p as *mut u32) = 99 }
}

#[inline(never)]
fn twice(r: &u32) -> u32 {
    let before = *r;
    sneaky(r);
    before + *r
}

fn main() {
    let a = 1;
    println!("twice(&a) = {}", twice(&a));
}
