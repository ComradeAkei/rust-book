//! Приложение C «Чек-лист ревью unsafe-кода».
//!
//! `before` — вектор фиксированной ёмкости на стеке в том виде, в каком
//! он пришёл на ревью: в нём четыре ошибки. Тесты, которые их показывают,
//! помечены `#[ignore]`: в них неопределённое поведение, и смысл они имеют
//! только под Miri. `after` — тот же тип после ревью, его тесты входят в
//! обычный прогон и в прогон Miri в CI.
//!
//! Тесты `before` запускайте по одному: Miri останавливается на первом же
//! неопределённом поведении, а без Miri порча памяти в одном тесте может
//! уронить весь тестовый процесс.
//!
//! ```text
//! cargo test -p appendix-c                                   # after
//! for t in push_past_capacity last_mut_through_shared panic_in_clone send_with_rc; do
//!     cargo +nightly miri test -p appendix-c -- --ignored --exact before::tests::before_$t
//! done                                                       # before под Miri
//! ```

pub mod after;
pub mod before;
