//! Вывод через semihosting без cortex-m-semihosting. `bkpt 0xAB`
//! останавливает процессор для отладчика; QEMU с `-semihosting` сам
//! выполняет просьбу: номер операции в r0, аргумент в r1 (справка — в
//! разделе 0.6).

use core::arch::asm;

// ANCHOR: sh
/// SYS_WRITE0: напечатать строку C по адресу в r1.
const SYS_WRITE0: usize = 0x04;
/// SYS_EXIT: завершить программу, причина в r1.
const SYS_EXIT: usize = 0x18;
/// ADP_Stopped_ApplicationExit — нормальный выход; QEMU вернёт 0.
const EXIT_OK: usize = 0x2_0026;
/// ADP_Stopped_RunTimeErrorUnknown — ошибка; QEMU вернёт 1.
const EXIT_FAIL: usize = 0x2_0023;

fn call(op: usize, arg: usize) {
    // SAFETY: bkpt 0xAB под QEMU выполняет операцию op; аргументы
    // проверил вызывающий. Память операция только читает.
    unsafe { asm!("bkpt 0xAB", inout("r0") op => _, in("r1") arg, options(nostack, readonly)) };
}

/// Строка без нуля в конце: кусками через буфер на стеке.
#[inline(never)]
pub fn put(s: &str) {
    let mut buf = [0u8; 64];
    for chunk in s.as_bytes().chunks(buf.len() - 1) {
        buf[..chunk.len()].copy_from_slice(chunk);
        buf[chunk.len()] = 0;
        call(SYS_WRITE0, buf.as_ptr() as usize);
    }
}

pub fn exit(ok: bool) -> ! {
    call(SYS_EXIT, if ok { EXIT_OK } else { EXIT_FAIL });
    // Под отладчиком без semihosting сюда можно попасть.
    loop {}
}
// ANCHOR_END: sh

/// Число в шестнадцатеричной записи, 8 цифр.
pub fn hex(x: u32) {
    let mut b = *b"0x00000000";
    for i in 0..8 {
        b[2 + i] = b"0123456789abcdef"[(x >> (28 - 4 * i)) as usize & 0xF];
    }
    put(ascii(&b));
}

/// Число в десятичной записи.
pub fn dec(mut x: u32) {
    let mut b = [0u8; 10];
    let mut i = b.len();
    loop {
        i -= 1;
        b[i] = b'0' + (x % 10) as u8;
        x /= 10;
        if x == 0 {
            break;
        }
    }
    put(ascii(&b[i..]));
}

/// Цифры как `&str`. `from_utf8(…).unwrap()` здесь не годится: ветка
/// ошибки ссылается на `Debug` для `Utf8Error`, и в прошивку попадает
/// `core::fmt` — несколько килобайт ради паники, которой не бывает.
fn ascii(b: &[u8]) -> &str {
    debug_assert!(b.is_ascii());
    // SAFETY: вызывающие передают только цифры ASCII и «0x».
    unsafe { core::str::from_utf8_unchecked(b) }
}
