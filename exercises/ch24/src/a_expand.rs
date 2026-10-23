//! Задача A: раскрытие `#[derive(Debug)]` и `#[entry]` из прошивки
//! `firmware/ch24-expand`.
//!
//! Сначала впишите в `predicted` раскрытие без компилятора: два `impl
//! Debug`, переходник, который `#[entry]` экспортирует под символом `main`,
//! и начало функции пользователя — сигнатуру и первый оператор. Затем
//! сверьте:
//!
//! ```text
//! cd firmware
//! RUSTC_BOOTSTRAP=1 cargo rustc -p ch24-expand --profile check -- -Zunpretty=expanded
//! ```
//!
//! или `cargo expand`, если он установлен. Тест сравнивает без учёта
//! пробелов, переводов строк и комментариев.

// ANCHOR: skeleton
pub fn predicted() -> [&'static str; 4] {
    todo!("раздел 24.7, задача A")
}
// ANCHOR_END: skeleton
