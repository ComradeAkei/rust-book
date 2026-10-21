//! Раздел A.5: коды возврата, errno и `Result`.

use std::ffi::c_int;
use std::fs::File;
use std::io;

// ANCHOR: errno
/// `int fd = open(path, O_RDONLY); if (fd < 0) { perror(path); ... }`
/// Ошибка приходит значением, errno уже прочитан и лежит внутри `io::Error`.
pub fn open_config(path: &str) -> io::Result<File> {
    File::open(path)
}
// ANCHOR_END: errno

// ANCHOR: last_os_error
unsafe extern "C" {
    /// `close(2)` из libc. Объявлена `unsafe`: закрыть чужой дескриптор,
    /// которым владеет, например, `File`, значит нарушить его инвариант.
    fn close(fd: c_int) -> c_int;
}

/// Закрывает заведомо неверный дескриптор и читает errno сразу после
/// вызова, пока его не перезаписал следующий. Безопасной функции,
/// закрывающей произвольный `fd`, быть не может: см. комментарий к `close`.
pub fn close_invalid() -> io::Error {
    // SAFETY: дескриптор -1 не может принадлежать ни одному объекту.
    let rc = unsafe { close(-1) };
    assert_eq!(rc, -1);
    io::Error::last_os_error()
}
// ANCHOR_END: last_os_error

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "Miri изолирует файловую систему")]
    fn errno_becomes_value() {
        let err = open_config("/nonexistent/telemetry.toml").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert_eq!(err.raw_os_error(), Some(2)); // ENOENT в Linux
        assert_eq!(err.to_string(), "No such file or directory (os error 2)");
    }

    #[test]
    #[cfg_attr(miri, ignore = "вызов функции libc")]
    fn close_bad_fd() {
        assert_eq!(close_invalid().raw_os_error(), Some(9)); // EBADF
    }
}
