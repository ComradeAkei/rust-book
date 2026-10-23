//! Задача B: один байт в UART0 тремя способами. Все три ждут места в FIFO
//! передатчика (бит TXFF регистра FR) и пишут байт в DR. Имена не
//! искажены: по ним тест находит тела в ассемблере.

use ch27_hal::pac;
use ch27_hal::serial::Serial;

// ANCHOR: raw
/// Как в C: адрес UART0, смещения регистров и номер бита — числами.
#[unsafe(no_mangle)]
pub fn ch27_b_raw(byte: u8) {
    const UART0: usize = 0x4000_C000;
    const DR: *mut u32 = UART0 as *mut u32;
    const FR: *const u32 = (UART0 + 0x18) as *const u32;
    const TXFF: u32 = 1 << 5;
    // SAFETY: FR и DR — регистры UART0, чтение FR побочных действий не
    // имеет, запись в DR отправляет байт.
    unsafe {
        while core::ptr::read_volatile(FR) & TXFF != 0 {}
        core::ptr::write_volatile(DR, u32::from(byte));
    }
}
// ANCHOR_END: raw

// ANCHOR: pac
/// Через PAC: имена регистров и полей из SVD.
#[unsafe(no_mangle)]
pub fn ch27_b_pac(byte: u8) {
    // SAFETY: функция только ждёт FIFO и пишет DR — то же, что делает
    // владелец UART0; в задаче другого владельца нет.
    let uart = unsafe { &*pac::Uart0::ptr() };
    while uart.fr().read().txff().bit_is_set() {}
    uart.dr().write(|w| w.data().set(byte));
}
// ANCHOR_END: pac

// ANCHOR: hal
/// Через HAL: `Serial` владеет UART0 и выводами, а запись — трейт
/// `embedded_io::Write`.
#[unsafe(no_mangle)]
pub fn ch27_b_hal(serial: &mut Serial, byte: u8) {
    let Ok(()) = embedded_io::Write::write_all(serial, &[byte]);
}
// ANCHOR_END: hal
