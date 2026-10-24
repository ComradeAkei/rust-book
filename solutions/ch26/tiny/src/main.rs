//! Задача 26.B: «Hello, world!» без std и libc — два системных вызова.
// ANCHOR: tiny
#![no_std]
#![no_main]

use core::arch::asm;

const MSG: &[u8] = b"Hello, world!\n";

/// Первая инструкция процесса. Функция не возвращается и ничего не
/// вызывает, поэтому выравнивание стека ей безразлично.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // SAFETY: write(1, MSG, len) читает только MSG; exit_group не
    // возвращается.
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") 1usize => _,
            in("rdi") 1usize,
            in("rsi") MSG.as_ptr(),
            in("rdx") MSG.len(),
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, readonly),
        );
        asm!("syscall", in("rax") 231usize, in("rdi") 0usize, options(noreturn, nostack));
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
// ANCHOR_END: tiny
