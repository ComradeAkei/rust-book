//! Примеры главы 24 «Процедурные макросы».
//!
//! - [`tokens`] — 24.1: дерево токенов, которое видит процедурный макрос.
//! - [`describe`] — 24.2: `#[derive(Describe)]` на syn и quote; 24.4:
//!   атрибуты-помощники `#[describe(...)]`.
//! - [`trace`] — 24.3: `#[traced]` и гигиена имён в атрибуте.
//! - [`device`] — 24.4: `#[interrupt]` с проверкой сигнатуры и списком
//!   прерываний устройства.
//!
//! Сами макросы живут в крейте `ch24-macros` и реэкспортируются отсюда.
//! Тесты ошибок компиляции (trybuild) и снимок раскрытия — в `tests/`.
#![cfg_attr(not(any(test, feature = "std")), no_std)]

// ANCHOR: self_alias
// Сгенерированный код ссылается на `::ch24::Describe`: у процедурного
// макроса нет `$crate`. Внутри самого крейта `ch24` такого пути нет —
// его даёт этот псевдоним.
extern crate self as ch24;
// ANCHOR_END: self_alias

pub mod describe;
pub mod device;
pub mod tokens;
#[cfg(feature = "std")]
pub mod trace;

pub use ch24_macros::{Describe, DescribeCallSite, token_tree, traced, traced_call_site};
pub use describe::{Describe, Field, dump_field, dump_header, layout};
