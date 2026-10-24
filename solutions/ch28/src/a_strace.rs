//! Задача A: какие системные вызовы делают шесть операций std. Программа
//! `ch28-ops` выполняет их по порядку, тест запускает её под `strace -f`
//! и сверяет предсказание с журналом.

// ANCHOR: api
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
// ANCHOR_END: api

// ANCHOR: predict
/// Имена системных вызовов операции, как их печатает strace. Порядок и
/// повторы не важны: тест проверяет, что каждый предсказанный вызов в
/// журнале есть и что главные вызовы операции предсказаны.
pub fn predict(op: Op) -> &'static [&'static str] {
    match op {
        // LineWriter: строка с переводом — один write в fd 1.
        Op::Println => &["write"],
        // open через openat(AT_FDCWD, ...) с O_CLOEXEC; metadata не нужна.
        Op::FileOpen => &["openat"],
        // Подсказка размера: statx и lseek(SEEK_CUR), затем read ровно на
        // остаток файла и ещё один read, который вернёт 0.
        Op::ReadToString => &["statx", "lseek", "read"],
        // output(): stdin — /dev/null, два канала под stdout и stderr,
        // posix_spawn — clone3 с CLONE_VFORK, в ребёнке dup2 и execve по
        // каталогам PATH; родитель читает каналы через poll и ждёт wait4.
        Op::CommandOutput => {
            &["openat", "pipe2", "clone3", "dup2", "execve", "poll", "read", "close", "wait4"]
        }
        // Стек нового потока — mmap и mprotect под сторожевую страницу,
        // clone3 с CLONE_THREAD, join — futex; поток ставит свой
        // sigaltstack и выходит через exit, не exit_group.
        Op::ThreadSpawnJoin => &["mmap", "mprotect", "clone3", "futex", "sigaltstack", "exit"],
        // clock_gettime через vDSO — ни одного перехода в ядро.
        Op::InstantNow => &[],
    }
}
// ANCHOR_END: predict
