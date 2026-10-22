//! Задача 0.B, часть 2: hello world без стандартной библиотеки Rust. Нужна
//! только libc: она запускает процесс, вызывает `main` и даёт `write`.

// ANCHOR: nostd
#![no_std]
#![no_main]

// Без атрибута link компоновщик не найдёт write: std больше не тянет libc за
// собой.
#[link(name = "c")]
unsafe extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn abort() -> !;
}

/// Точка входа в смысле C: её вызывает `__libc_start_main` из crt1.o.
#[unsafe(no_mangle)]
extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let msg = b"Hello, world!\n";
    // SAFETY: буфер живёт до конца вызова, длина взята из самого буфера,
    // дескриптор 1 открыт процессом-родителем.
    let n = unsafe { write(1, msg.as_ptr(), msg.len()) };
    if n == msg.len() as isize { 0 } else { 1 }
}

/// Паниковать здесь нечему, но обработчик обязан быть. Процесс под Linux
/// должен завершиться, а не крутиться в пустом цикле, поэтому зовём abort из
/// libc: процесс получит SIGABRT.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // SAFETY: abort не принимает аргументов и не возвращается.
    unsafe { abort() }
}
// ANCHOR_END: nostd
