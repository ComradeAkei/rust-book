//! Глава 8: прошивки разделов 8.3, 8.5 и 8.8.
//!
//! - `target` вызывает `ch08_bump`, у которой две реализации — с атомиками
//!   и с запретом прерываний, — и печатает, что сгенерировал `build.rs`;
//! - `boot` хранит счётчик загрузок в секции `.uninit`, которую не
//!   обнуляет код запуска, и перезагружает микроконтроллер.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch08-sections --bin target                               # Cortex-M3
//! cargo run --release -p ch08-sections --bin target --target thumbv6m-none-eabi   # Cortex-M0
//! cargo run --release -p ch08-sections --bin boot
//! ```
#![no_std]

use cortex_m_semihosting::debug;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
