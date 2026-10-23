// Раздел 8.7: два одинаковых имени внутри одного крейта.
// ANCHOR: same
#[unsafe(no_mangle)]
pub extern "C" fn station_reset() -> u32 {
    1
}

mod dma {
    #[unsafe(no_mangle)]
    pub extern "C" fn station_reset() -> u32 {
        2
    }
}
// ANCHOR_END: same
