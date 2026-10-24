//! Задача B: один байт в UART0 тремя способами. Все три ждут места в FIFO
//! передатчика (бит TXFF регистра FR) и пишут байт в DR. Имена не
//! искажены: по ним тест находит тела в ассемблере.

use ch27_hal::pac;
use ch27_hal::serial::Serial;

/// Как в C: адрес UART0 (0x4000_C000), смещения регистров и номер бита —
/// числами, `read_volatile` и `write_volatile`.
#[unsafe(no_mangle)]
pub fn ch27_b_raw(byte: u8) {
    let _ = byte;
    todo!("задача B: числами")
}

/// Через PAC: `pac::Uart0::ptr()`, регистры `fr` и `dr`, поля `txff` и
/// `data`.
#[unsafe(no_mangle)]
pub fn ch27_b_pac(byte: u8) {
    let _ = (byte, pac::Uart0::ptr());
    todo!("задача B: через PAC")
}

/// Через HAL: трейт `embedded_io::Write` у `Serial`.
#[unsafe(no_mangle)]
pub fn ch27_b_hal(serial: &mut Serial, byte: u8) {
    let _ = (serial, byte);
    todo!("задача B: через HAL")
}
