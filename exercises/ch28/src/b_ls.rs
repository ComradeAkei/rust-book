//! Задача B: `ls -la` одного каталога без `std::fs` — getdents64 через
//! `rustix::fs::RawDir`, `fstatat` без перехода по ссылкам, `readlinkat`,
//! имена владельцев из /etc/passwd и /etc/group. Вывод должен совпасть
//! байт в байт с `LC_ALL=C TZ=UTC ls -la --time-style='+%Y-%m-%d %H:%M'`.
//! Устройства (`b`, `c`) в задачу не входят.

use std::io;
use std::path::Path;

// ANCHOR: list
/// Вывод `ls -la` для каталога `dir`: строка `total N` и по строке на
/// запись, включая `.` и `..`, в порядке байтов имени.
pub fn list(dir: &Path) -> io::Result<String> {
    let _ = dir;
    todo!("задача B: list")
}
// ANCHOR_END: list
