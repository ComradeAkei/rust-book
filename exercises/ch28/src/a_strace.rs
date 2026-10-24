//! Задача A: какие системные вызовы делают шесть операций std. Программа
//! `ch28-ops` выполняет их по порядку, тест запускает её под `strace -f`
//! и сверяет предсказание с журналом.

/// Операции программы `ch28-ops` в порядке выполнения.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `println!("строка")`.
    Println,
    /// `File::open("/etc/passwd")`.
    FileOpen,
    /// `read_to_string` из открытого файла.
    ReadToString,
    /// `Command::new("true").output()` — вместе с вызовами ребёнка.
    CommandOutput,
    /// `thread::spawn(..).join()` — вместе с вызовами нового потока.
    ThreadSpawnJoin,
    /// `Instant::now()`.
    InstantNow,
}

pub const OPS: [Op; 6] = [
    Op::Println,
    Op::FileOpen,
    Op::ReadToString,
    Op::CommandOutput,
    Op::ThreadSpawnJoin,
    Op::InstantNow,
];

// ANCHOR: predict
/// Имена системных вызовов операции, как их печатает strace. Порядок и
/// повторы не важны: тест проверяет, что каждый предсказанный вызов в
/// журнале есть и что главные вызовы операции предсказаны. Сначала
/// предскажите, потом запустите `strace -f` и объясните расхождения.
pub fn predict(op: Op) -> &'static [&'static str] {
    let _ = op;
    todo!("задача A: predict")
}
// ANCHOR_END: predict
