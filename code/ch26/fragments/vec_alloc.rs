// Раздел 26.1: Vec живёт в alloc, крейт надо подключить явно.
// ANCHOR: vec_alloc
#![no_std]

extern crate alloc;

use alloc::vec::Vec;

pub fn readings() -> usize {
    let v: Vec<u16> = Vec::new();
    v.len()
}
// ANCHOR_END: vec_alloc
