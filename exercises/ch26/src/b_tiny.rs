//! Задача B: программа — отдельный крейт `tiny/` со своим workspace, его
//! собирает и запускает общий тест.

/// Каталог крейта `tiny`.
pub const TINY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tiny");
