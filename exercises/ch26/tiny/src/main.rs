//! Задача 26.B: «Hello, world!» без std и libc.
#![no_std]
#![no_main]

// TODO(задача B): точка входа `_start`: write(1, "Hello, world!\n", 14) и
// exit_group(0) инструкцией syscall (раздел 26.2).

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
