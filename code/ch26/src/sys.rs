//! Раздел 26.2: системные вызовы Linux на x86-64 без libc.
//!
//! Соглашение ядра: номер вызова в `rax`, аргументы в `rdi`, `rsi`,
//! `rdx`, `r10`, `r8`, `r9`, результат в `rax`. Инструкция `syscall`
//! портит `rcx` (туда процессор кладёт адрес возврата) и `r11` (туда —
//! `rflags`). Ошибка — отрицательное число от −4095 до −1, это `-errno`:
//! переменной `errno` у ядра нет, её заводит libc.
//!
//! Тот же файл подключает программа `code/ch26/start` через `#[path]`.

use core::arch::asm;

pub const WRITE: usize = 1;
pub const GETPID: usize = 39;
pub const EXIT_GROUP: usize = 231;

/// Код ошибки ядра, положительный: `EBADF` = 9.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Errno(pub i32);

impl Errno {
    pub const EBADF: Errno = Errno(9);
    pub const EFAULT: Errno = Errno(14);
}

// ANCHOR: syscall
/// Системный вызов с тремя аргументами.
///
/// # Safety
///
/// Аргументы должны быть допустимы для вызова `nr`: указатели — на
/// живую память нужного размера и так далее. Ядро проверит только то,
/// что не ведёт к его собственной ошибке.
#[inline]
pub unsafe fn syscall3(nr: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let ret: isize;
    // SAFETY: инструкция syscall портит только rcx и r11, стек не трогает;
    // остальное — договор вызывающего.
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") nr as isize => ret,
            in("rdi") a0,
            in("rsi") a1,
            in("rdx") a2,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    ret
}
// ANCHOR_END: syscall

// ANCHOR: wrappers
fn check(ret: isize) -> Result<usize, Errno> {
    if (-4095..0).contains(&ret) { Err(Errno(-ret as i32)) } else { Ok(ret as usize) }
}

/// `write(2)`: сколько байт принято, или код ошибки.
pub fn write(fd: i32, buf: &[u8]) -> Result<usize, Errno> {
    // SAFETY: указатель и длина взяты из одного среза.
    check(unsafe { syscall3(WRITE, fd as usize, buf.as_ptr() as usize, buf.len()) })
}

/// Весь срез, повторяя `write` после частичной записи.
pub fn write_all(fd: i32, mut buf: &[u8]) -> Result<(), Errno> {
    while !buf.is_empty() {
        let n = write(fd, buf)?;
        buf = &buf[n..];
    }
    Ok(())
}

pub fn getpid() -> i32 {
    // SAFETY: у getpid нет аргументов, и он не ошибается.
    unsafe { syscall3(GETPID, 0, 0, 0) as i32 }
}

/// Завершить процесс: `exit_group`, а не `exit` (60) — тот завершает
/// только вызвавший поток. `_exit` из libc — это тоже `exit_group`.
pub fn exit(code: i32) -> ! {
    // SAFETY: вызов не возвращается, аргумент — любое число.
    unsafe {
        asm!("syscall", in("rax") EXIT_GROUP, in("rdi") code as usize, options(noreturn, nostack));
    }
}
// ANCHOR_END: wrappers

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::io::{Read, Seek};
    use std::os::fd::AsRawFd;

    #[test]
    #[cfg_attr(miri, ignore = "Miri не выполняет ассемблерные вставки")]
    fn write_goes_to_the_file() {
        let mut f = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(std::env::temp_dir().join(std::format!("ch26-sys-{}", std::process::id())))
            .unwrap();
        assert_eq!(write(f.as_raw_fd(), b"kernel"), Ok(6));
        write_all(f.as_raw_fd(), b", no libc").unwrap();
        f.rewind().unwrap();
        let mut s = std::string::String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "kernel, no libc");
    }

    #[test]
    #[cfg_attr(miri, ignore = "Miri не выполняет ассемблерные вставки")]
    fn errors_are_negative_errno() {
        assert_eq!(write(-1, b"x"), Err(Errno::EBADF));
        // SAFETY: адрес 8 не отображён: ядро вернёт EFAULT, а не упадёт.
        let r = unsafe { syscall3(WRITE, 1, 8, 1) };
        assert_eq!(r, -14);
        assert_eq!(check(r), Err(Errno::EFAULT));
    }

    #[test]
    #[cfg_attr(miri, ignore = "Miri не выполняет ассемблерные вставки")]
    fn getpid_matches_std() {
        assert_eq!(getpid() as u32, std::process::id());
    }
}
