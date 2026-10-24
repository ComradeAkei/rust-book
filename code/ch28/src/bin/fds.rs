//! Раздел 28.2: опыты с дескрипторами для strace и для отладочной проверки
//! std.
//!
//! - `fds copy SRC DST` — копия файла; под strace видно, что на пути
//!   ошибки дескриптор источника закрыт.
//! - `fds double-close` — два `OwnedFd` на один номер. В отладочной сборке
//!   std ловит второе закрытие и завершает процесс.

use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["copy", src, dst] => match ch28::fd::copy(Path::new(src), Path::new(dst)) {
            Ok(n) => println!("скопировано {n} байт"),
            Err(e) => {
                eprintln!("ошибка: {e}");
                std::process::exit(1);
            }
        },
        // ANCHOR: double
        ["double-close"] => {
            let raw = ch28::fd::memfd_with("ch28-double", b"x").unwrap().into_raw_fd();
            // SAFETY: нарушено намеренно — у номера два владельца.
            let a = unsafe { OwnedFd::from_raw_fd(raw) };
            let b = unsafe { OwnedFd::from_raw_fd(raw) };
            drop(a);
            eprintln!("первый владелец закрыл {raw}");
            drop(b);
            eprintln!("второй владелец закрыл {raw}");
        }
        // ANCHOR_END: double
        _ => {
            eprintln!("использование: fds copy SRC DST | fds double-close");
            std::process::exit(2);
        }
    }
}
