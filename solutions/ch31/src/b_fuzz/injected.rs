//! Заготовка задачи с внедрёнными ошибками: файлы берутся прямо из
//! `exercises/ch31/src/b_fuzz/`. С теми же оракулами цели фаззинга
//! `packet_injected` и `config_injected` находят обе ошибки.

#[path = "../../../../exercises/ch31/src/b_fuzz/ip.rs"]
pub mod ip;
#[path = "../../../../exercises/ch31/src/b_fuzz/lexer.rs"]
pub mod lexer;

include!("oracles.rs");
