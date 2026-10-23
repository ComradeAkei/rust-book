//! Глава 7, раздел 7.10: ошибки протокола из `project/telemetry-frame` на
//! Cortex-M.
//!
//! Обе прошивки разбирают один и тот же испорченный кадр и выбирают реакцию
//! по `FrameError::recovery`. `quiet` печатает только строковые литералы,
//! `verbose` вдобавок выводит сообщения ошибок через `Display`. Разница в
//! размере — цена сообщений для прошивки.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch07-frame --bin verbose
//! cargo run --release -p ch07-frame --bin quiet
//! ```
#![no_std]

use cortex_m_semihosting::{debug, hprintln};
use telemetry_frame::{Recovery, RxError, decode_at};

// ANCHOR: stream
/// Два подтверждения подряд: с номером 255 (тот же кадр, что `ACK` в
/// общих тестах проекта) и с номером 0, у которого испорчена CRC.
pub static STREAM: [u8; 16] = [
    0xa5, 0x01, 0x03, 0xff, 0x00, 0x00, 0xd7, 0x52, // CRC верна
    0xa5, 0x01, 0x03, 0x00, 0x00, 0x00, 0x55, 0x55, // CRC испорчена
];

/// Разбирает кадр со смещения 8 и печатает, что делать дальше. Смещение
/// спрятано от оптимизатора, как если бы его вычислил приёмник.
pub fn second_frame() -> Result<(), RxError> {
    let offset = core::hint::black_box(8);
    let e = match decode_at(&STREAM, offset) {
        Ok(_) => return Ok(()),
        Err(e) => e,
    };
    match e.error.recovery() {
        Recovery::NeedMore(_) => hprintln!("ждать продолжения"),
        Recovery::Resync => hprintln!("искать маркер со следующего байта"),
        Recovery::Drop => hprintln!("пропустить кадр"),
        Recovery::Reject => hprintln!("ошибка отправителя"),
    }
    Err(e)
}
// ANCHOR_END: stream

/// Без форматирования: паника здесь не ожидается, а `core::fmt` не должен
/// попасть в `quiet` через обработчик.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
