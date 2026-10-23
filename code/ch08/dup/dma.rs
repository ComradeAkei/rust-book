// Раздел 8.7: крейт драйвера DMA выбрал то же имя.
// ANCHOR: dma
#[unsafe(no_mangle)]
pub extern "C" fn station_reset() -> u32 {
    2
}
// ANCHOR_END: dma
