//! Задача D, часть 1: паника в деструкторе во время раскрутки другой
//! паники. Процесс завершается через abort, и деструкторы остальных
//! значений уже не выполняются.

// ANCHOR: double
struct Port(&'static str);

impl Drop for Port {
    fn drop(&mut self) {
        eprintln!("закрываю {}", self.0);
        if std::thread::panicking() {
            panic!("ошибка при закрытии {}", self.0);
        }
    }
}

fn main() {
    let _log = Port("журнал");
    let _uart = Port("uart0");
    panic!("датчик не отвечает");
}
// ANCHOR_END: double
