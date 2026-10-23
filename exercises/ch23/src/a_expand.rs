//! Задача A: раскрытие шести вызовов из `code/ch23/fragments/a_calls.rs`.
//!
//! Сначала впишите в `predicted` раскрытие каждого вызова без компилятора:
//! тела функций `call1`…`call5` и блок `impl Point`, который даёт
//! `getters!`. Затем сверьте:
//!
//! ```text
//! cd code/ch23/fragments
//! RUSTC_BOOTSTRAP=1 rustc --edition 2024 --crate-type lib -Zunpretty=expanded a_calls.rs
//! ```
//!
//! или `cargo expand`, если он установлен. Тест сравнивает без учёта
//! пробелов и переводов строк.

// ANCHOR: skeleton
pub fn predicted() -> [&'static str; 6] {
    todo!("раздел 23.7, задача A")
}
// ANCHOR_END: skeleton
