//! Данные задачи 0.A «Какие функции доживут до release-бинарника».
//!
//! Сами функции лежат в `examples/vanish.rs`. Студент записывает прогнозы в
//! `exercises/ch00/src/a_vanish.rs`, а общий тест собирает пример в release,
//! читает таблицу символов через `nm` и сравнивает её с прогнозами.

/// Имена пяти функций в порядке их номеров в `examples/vanish.rs`.
pub const FUNCTIONS: [&str; 5] = ["crc_step", "checksum", "popcount", "ch00a_exported", "triple"];
