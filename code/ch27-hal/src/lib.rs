//! Раздел 27.5: маленький HAL для LM3S6965 поверх PAC `ch27-pac`.
//!
//! - [`sysctl`] — включение тактов периферии и частота ядра;
//! - [`gpio`] — выводы с режимом в типе: `Pin<PORT, N, MODE>` нулевого
//!   размера, трейты `OutputPin`, `StatefulOutputPin`, `InputPin`;
//! - [`serial`] — UART0 на выводах PA0 и PA1, трейты `embedded_io::Read`
//!   и `Write`;
//! - [`timer`] — таймер общего назначения в периодическом режиме;
//! - [`adc`] — последовательность 3 АЦП, запуск от таймера;
//! - [`delay`] — задержка по SysTick, трейт `DelayNs`.
//!
//! HAL собирается и на хосте: тесты проверяют адреса и делители, а
//! регистры трогают только прошивки.
#![no_std]

pub use ch27_pac as pac;

pub mod adc;
pub mod delay;
pub mod gpio;
#[cfg(feature = "probe")]
pub mod probe;
pub mod serial;
pub mod sysctl;
pub mod timer;

/// Трейты, без которых методы HAL не видны.
pub mod prelude {
    pub use crate::gpio::GpioExt as _;
    pub use embedded_hal::delay::DelayNs as _;
    pub use embedded_hal::digital::{InputPin as _, OutputPin as _, StatefulOutputPin as _};
    pub use embedded_io::{Read as _, Write as _};
}
