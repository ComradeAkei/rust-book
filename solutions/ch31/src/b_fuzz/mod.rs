//! Задача 31.B: фаззинг разбора IPv4 и TCP из главы 6 и токенизатора
//! конфигурации из главы 5.
//!
//! [`ip`] и [`lexer`] — исправленные копии; [`injected`] — заготовка с
//! внедрёнными ошибками, те же файлы, что в `exercises/ch31`. Оракулы —
//! [`check_packet`] и [`check_config`]: они паникуют, если разбор нарушил
//! свойство, и их зовут цели фаззинга из `solutions/ch31/fuzz`.

pub mod injected;
pub mod ip;
pub mod lexer;

include!("oracles.rs");
