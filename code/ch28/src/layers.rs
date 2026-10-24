//! Раздел 28.1: один системный вызов на четырёх уровнях — std, libc, rustix
//! и nix. Программа `layers` вызывает их по очереди; тест запускает её под
//! strace и проверяет, что каждый уровень — ровно один `write(1, …)`.

use std::io::{self, Write};

// ANCHOR: std
/// std: `Stdout` — общий буфер строк под мьютексом. Строка, которая
/// кончается переводом строки, уходит одним `write`.
pub fn say_std(msg: &str) -> io::Result<()> {
    io::stdout().lock().write_all(msg.as_bytes())
}
// ANCHOR_END: std

// ANCHOR: libc
/// libc: обёртка glibc. Ошибка — `-1` и `errno`; короткую запись и EINTR
/// вызывающий обрабатывает сам.
pub fn say_libc(msg: &str) -> io::Result<()> {
    let mut rest = msg.as_bytes();
    while !rest.is_empty() {
        // SAFETY: указатель и длина — от живого среза, fd 1 открыт всё время
        // работы программы.
        let n = unsafe { libc::write(libc::STDOUT_FILENO, rest.as_ptr().cast(), rest.len()) };
        if n < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        rest = &rest[n as usize..];
    }
    Ok(())
}
// ANCHOR_END: libc

// ANCHOR: rustix
/// rustix: безопасная обёртка, дескриптор — `BorrowedFd`, ошибка —
/// `Errno` в два байта. На x86-64 Linux вызов идёт мимо libc, инструкцией
/// `syscall` прямо в теле функции.
pub fn say_rustix(msg: &str) -> io::Result<()> {
    let mut rest = msg.as_bytes();
    while !rest.is_empty() {
        match rustix::io::write(rustix::stdio::stdout(), rest) {
            Ok(n) => rest = &rest[n..],
            Err(rustix::io::Errno::INTR) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
// ANCHOR_END: rustix

// ANCHOR: nix
/// nix: безопасная обёртка над libc, ошибка — `Errno` как перечисление.
pub fn say_nix(msg: &str) -> io::Result<()> {
    let mut rest = msg.as_bytes();
    while !rest.is_empty() {
        match nix::unistd::write(io::stdout(), rest) {
            Ok(n) => rest = &rest[n..],
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
// ANCHOR_END: nix

// ANCHOR: errors
/// Одна и та же ошибка `open` — ENOENT — на четырёх уровнях: что
/// печатает `Display` и сколько места занимает значение ошибки.
pub fn open_errors(path: &str) -> [(&'static str, String, usize); 4] {
    let std_err = std::fs::File::open(path).unwrap_err();

    let c = std::ffi::CString::new(path).unwrap();
    // SAFETY: строка с нулём на конце живёт до конца вызова.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    assert_eq!(fd, -1, "файл неожиданно нашёлся");
    let libc_err = io::Error::last_os_error();

    use rustix::fs::{Mode, OFlags};
    let rustix_err = rustix::fs::open(path, OFlags::RDONLY | OFlags::CLOEXEC, Mode::empty())
        .map(drop)
        .unwrap_err();

    let nix_err = nix::fcntl::open(
        path,
        nix::fcntl::OFlag::O_RDONLY | nix::fcntl::OFlag::O_CLOEXEC,
        nix::sys::stat::Mode::empty(),
    )
    .map(drop)
    .unwrap_err();

    [
        ("std", std_err.to_string(), size_of::<io::Error>()),
        ("libc", libc_err.to_string(), size_of::<i32>()),
        ("rustix", rustix_err.to_string(), size_of::<rustix::io::Errno>()),
        ("nix", nix_err.to_string(), size_of::<nix::errno::Errno>()),
    ]
}
// ANCHOR_END: errors

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_layers_one_errno() {
        let errs = open_errors("/нет/такого/файла");
        for (layer, text, _) in &errs {
            assert!(text.contains("No such file or directory"), "{layer}: {text}");
        }
        // Errno — номер, а io::Error — указатель с тегом.
        assert_eq!(errs.map(|e| e.2), [8, 4, 2, 4]);
    }
}
