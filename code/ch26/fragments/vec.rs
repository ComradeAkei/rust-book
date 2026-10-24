// Раздел 26.1: в no_std нет Vec — ни в прелюдии, ни под этим именем.
// ANCHOR: vec
#![no_std]

pub fn readings() -> usize {
    let v: Vec<u16> = Vec::new();
    v.len()
}
// ANCHOR_END: vec
