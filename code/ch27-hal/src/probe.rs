//! Функции с неискажёнными именами для `tests/asm.rs`: одна и та же
//! запись тремя способами — сырым указателем, через PAC и через HAL, —
//! смена режима вывода и драйвер 74HC595 поверх выводов HAL. Собираются
//! только с возможностью `probe`.

use crate::gpio::{Input, Output, PB0, PB1, PB2, PF0};
use crate::pac;
use ch27::hc595::Hc595;
use embedded_hal::digital::OutputPin;

// ANCHOR: three
/// PF0 в высокий уровень сырым указателем, как в C: слово DATA по
/// адресу маски 1 << 0, то есть смещение 4.
#[unsafe(no_mangle)]
pub fn ch27_raw_set_high() {
    // SAFETY: 0x4002_5004 — регистр DATA порта F с маской вывода 0.
    unsafe { core::ptr::write_volatile(0x4002_5004 as *mut u32, 1) }
}

/// То же через PAC: имя регистра и поля вместо чисел.
#[unsafe(no_mangle)]
pub fn ch27_pac_set_high() {
    // SAFETY: только запись в DATA порта F с маской вывода 0.
    let gpiof = unsafe { &*pac::Gpiof::ptr() };
    gpiof.data(1).write(|w| w.p(0).set_bit());
}

/// То же через HAL: вывод — значение нулевого размера с режимом в типе.
#[unsafe(no_mangle)]
pub fn ch27_hal_set_high(pin: &mut PF0<Output>) {
    let Ok(()) = pin.set_high();
}
// ANCHOR_END: three

#[unsafe(no_mangle)]
pub fn ch27_hal_into_output(pin: PF0<Input>) -> PF0<Output> {
    pin.into_output()
}

#[unsafe(no_mangle)]
pub fn ch27_hc595_write(leds: &mut Hc595<PB0<Output>, PB1<Output>, PB2<Output>>, byte: u8) {
    let Ok(()) = leds.write(byte);
}
