// Раздел 8.7: крейт драйвера UART экспортирует функцию для C.
// ANCHOR: uart
#[unsafe(no_mangle)]
pub extern "C" fn station_reset() -> u32 {
    1
}
// ANCHOR_END: uart
