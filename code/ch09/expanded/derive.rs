// Раздел 9.6: что генерирует derive. Снимок рядом, в derive.out.rs:
// RUSTC_BOOTSTRAP=1 rustc --edition 2024 --crate-type lib -Zunpretty=expanded derive.rs
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sample {
    pub sensor: u16,
    pub value: i32,
}
