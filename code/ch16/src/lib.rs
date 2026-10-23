//! Примеры главы 16 «unsafe и неопределённое поведение».
//!
//! Весь код здесь корректен и проходит Miri в обеих моделях алиасинга
//! (Stacked Borrows и Tree Borrows). Примеры неопределённого поведения
//! живут отдельно — в задаче A (`solutions/ch16`), где каждый фрагмент
//! помечен `#[ignore]` и запускается под Miri по одному, чтобы Miri его
//! поймал, а обычный `cargo test` не выполнял UB.
//!
//! Фрагменты для книги отмечены парами `ANCHOR: имя` / `ANCHOR_END: имя`.
//! Программы, которые компилятор отвергает, лежат в doc-тестах
//! `compile_fail` с кодом ошибки.

pub mod abilities;
pub mod aliasing;
pub mod codegen;
pub mod invariants;
pub mod provenance;
pub mod safe_api;
