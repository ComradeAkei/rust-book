//! Раздел 7.4: `std::io::Error` — пример компактной ошибки стандартной
//! библиотеки. Модуль только для хоста: в `no_std` нет `std::io`.

use std::fs::File;
use std::io::{self, ErrorKind, Read};

// ANCHOR: io
/// Прочитать первые байты файла. Ошибка открытия и ошибка чтения — один
/// тип `io::Error`, у которого есть вид (`kind`) и код ОС.
pub fn head(path: &str, buf: &mut [u8]) -> io::Result<usize> {
    let mut f = File::open(path)?;
    f.read(buf)
}

pub fn describe(e: &io::Error) -> String {
    match (e.kind(), e.raw_os_error()) {
        (ErrorKind::NotFound, _) => "нет такого файла".to_string(),
        (_, Some(code)) => format!("ошибка ОС {code}: {e}"),
        (kind, None) => format!("{kind:?}: {e}"),
    }
}
// ANCHOR_END: io

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn io_error_is_one_word() {
        assert_eq!(size_of::<io::Error>(), size_of::<usize>());
        assert_eq!(size_of::<io::Result<()>>(), size_of::<usize>());
    }

    #[test]
    #[cfg_attr(miri, ignore = "обращается к файловой системе")]
    fn errors_keep_os_code() {
        let mut b = [0; 4];
        let e = head("/nonexistent/station.ini", &mut b).unwrap_err();
        assert_eq!(describe(&e), "нет такого файла");
        assert_eq!(e.raw_os_error(), Some(2));
        let e = head("/", &mut b).unwrap_err();
        assert_eq!(e.raw_os_error(), Some(21)); // EISDIR
        assert_eq!(describe(&e), "ошибка ОС 21: Is a directory (os error 21)");
    }
}
