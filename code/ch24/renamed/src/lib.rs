//! Крейт `ch24` подключён под именем `station` (см. Cargo.toml). Код из
//! `#[derive(Describe)]` по умолчанию ссылается на `::ch24::...`, а такого
//! крейта здесь нет: путь к нему пользователь называет сам.
#![no_std]

// ANCHOR: renamed
#[derive(station::Describe)]
#[describe(crate = station)]
#[repr(C)]
pub struct Frame {
    pub id: u8,
    pub len: u8,
}
// ANCHOR_END: renamed

// ANCHOR: broken
#[cfg(feature = "broken")]
#[derive(station::Describe)]
pub struct Broken {
    pub id: u8,
}
// ANCHOR_END: broken
