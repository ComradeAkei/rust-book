//! Сеанс в госте, похожий на работу в оболочке: каждая команда печатает
//! строку `# команда`, свой результат и новые записи журнала ядра.
//! Такой вывод и попадает в текст главы.

use crate::guest::{Kmsg, drain_console, insmod, rmmod};
use crate::kmsg::level_name;
use std::fs;
use std::io;
use std::path::Path;

/// Первая строка вывода гостя: всё, что раньше, — BIOS и загрузчик.
pub const START: &str = "### ch29: начало";
/// Последняя строка: сценарий дошёл до конца.
pub const END: &str = "### ch29: конец";

/// Вывод гостя между [`START`] и [`END`]; `None` — гость до конца не дошёл.
pub fn guest_part(serial: &str) -> Option<&str> {
    let (_, rest) = serial.split_once(START)?;
    let (body, _) = rest.split_once(END)?;
    Some(body.trim_start_matches('\n'))
}

/// Строки без консоли ядра. Пока init печатает сеанс, консоль молчит
/// (`loglevel=1`), но oops и WARN на время отчёта поднимают её уровень, и
/// тот же отчёт выходит дважды: строками `[    1.234567] …` прямо от ядра
/// и записями из `/dev/kmsg`, которые печатает сеанс. Оставляем вторые.
pub fn without_console(body: &str) -> String {
    let console = |l: &str| {
        l.strip_prefix('[').and_then(|r| r.split_once("] ")).is_some_and(|(t, _)| {
            t.trim_start().split_once('.').is_some_and(|(a, b)| {
                !a.is_empty()
                    && a.bytes().all(|c| c.is_ascii_digit())
                    && b.len() == 6
                    && b.bytes().all(|c| c.is_ascii_digit())
            })
        })
    };
    body.lines().filter(|l| !console(l)).map(|l| format!("{l}\n")).collect()
}

pub struct Session {
    kmsg: Kmsg,
    /// Записи журнала, которые печатать не нужно.
    quiet: Vec<String>,
}

/// Записи, которые ядро печатает само, в своё время, а не в ответ на
/// команду сеанса: уточнение частоты TSC и смена источника времени через
/// секунду-другую после загрузки, жалобы таймеров и workqueue на
/// медленную машину-хозяина. Когда хозяин нагружен (параллельные тесты,
/// CI), они попадают между `insmod` и откликом модуля.
const NOISE: [&str; 4] = [
    "tsc: Refined TSC clocksource calibration",
    "clocksource: ",
    "hrtimer: interrupt took",
    "hogged CPU for",
];

impl Session {
    /// Открывает журнал и пропускает всё, что ядро напечатало при загрузке,
    /// а потом — записи из `NOISE`.
    pub fn start() -> io::Result<Session> {
        crate::guest::setup()?;
        let mut kmsg = Kmsg::open()?;
        kmsg.drain();
        println!("{START}");
        Ok(Session { kmsg, quiet: NOISE.map(String::from).to_vec() })
    }

    /// Не печатать записи, текст которых содержит `s`.
    pub fn hide(&mut self, s: &str) {
        self.quiet.push(s.to_string());
    }

    /// Печатает новые записи журнала, как `dmesg -x`.
    pub fn dmesg(&mut self) {
        for r in self.kmsg.drain() {
            if self.quiet.iter().any(|q| r.text.contains(q.as_str())) {
                continue;
            }
            println!("{:<7} {}", format!("{}:", level_name(r.level)), r.text);
        }
    }

    /// Ждёт запись с `needle` и печатает всё новое.
    pub fn dmesg_until(&mut self, needle: &str, secs: u64) {
        for r in self.kmsg.wait_for(needle, std::time::Duration::from_secs(secs)) {
            if self.quiet.iter().any(|q| r.text.contains(q.as_str())) {
                continue;
            }
            println!("{:<7} {}", format!("{}:", level_name(r.level)), r.text);
        }
    }

    pub fn insmod(&mut self, module: &str, params: &str) -> io::Result<()> {
        let path = format!("/lib/{module}.ko");
        println!("# insmod {path}{}{params}", if params.is_empty() { "" } else { " " });
        drain_console();
        let r = insmod(&path, params);
        if let Err(e) = &r {
            println!("insmod: {e}");
        }
        self.dmesg();
        r
    }

    pub fn rmmod(&mut self, module: &str) -> io::Result<()> {
        println!("# rmmod {module}");
        drain_console();
        let r = rmmod(module);
        if let Err(e) = &r {
            println!("rmmod: {e}");
        }
        self.dmesg();
        r
    }

    /// `cat` файла, строки которого содержат `filter`.
    pub fn grep(&mut self, filter: &str, path: &str) {
        println!("# grep {filter} {path}");
        match fs::read_to_string(path) {
            Ok(s) => s.lines().filter(|l| l.contains(filter)).for_each(|l| println!("{l}")),
            Err(e) => println!("grep: {path}: {e}"),
        }
    }

    pub fn cat(&mut self, path: &str) {
        println!("# cat {path}");
        match fs::read_to_string(path) {
            Ok(s) => print!("{s}"),
            Err(e) => println!("cat: {path}: {e}"),
        }
    }

    /// Есть ли файл — как `ls -d`.
    pub fn exists(&mut self, path: &str) {
        let what = if Path::new(path).exists() { "есть" } else { "нет" };
        println!("# ls -d {path}: {what}");
    }

    pub fn say(&mut self, text: &str) {
        println!("{text}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_lines_are_dropped() {
        let body = "# insmod /lib/x.ko\n[    1.141415] rust_kernel: panicked at x.rs:1:1:\n\
                    emerg:  rust_kernel: panicked at x.rs:1:1:\n[12345.000001] Oops\n[ не время ] строка\n";
        assert_eq!(
            without_console(body),
            "# insmod /lib/x.ko\nemerg:  rust_kernel: panicked at x.rs:1:1:\n[ не время ] строка\n"
        );
        assert_eq!(guest_part("BIOS\n### ch29: начало\nтело\n### ch29: конец\n"), Some("тело\n"));
        assert_eq!(guest_part("### ch29: начало\nпаника"), None);
    }
}
