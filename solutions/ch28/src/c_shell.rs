//! Задача C: мини-оболочка. Конвейеры `a | b | c`, перенаправления `<`,
//! `>`, `>>`, фоновые задания `&` и встроенные `cd` и `exit`. Каждое
//! задание — своя группа процессов; в интерактивном режиме оболочка
//! отдаёт терминал группе задания, и Ctrl-C прерывает только его.
//!
//! Чего нет: подстановок `$`, `*` и `~`, `;` и `&&`, Ctrl-Z с `fg` и `bg`
//! — задания игнорируют SIGTSTP, как его игнорирует сама оболочка.

use rustix::pipe::{PipeFlags, pipe_with};
use rustix::process::{Pid, getpgrp, getpid, setpgid};
use rustix::termios::{isatty, tcsetpgrp};
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};

// ANCHOR: types
/// Одна команда конвейера.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stage {
    pub argv: Vec<String>,
    /// `< файл`.
    pub input: Option<String>,
    /// `> файл` или `>> файл`.
    pub output: Option<Redirect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redirect {
    pub path: String,
    pub append: bool,
}

/// Строка оболочки: конвейер и признак фонового задания.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub stages: Vec<Stage>,
    pub background: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Кавычка без пары.
    Unterminated,
    /// Пустая команда: `a | | b`, `| a`, `a |`, `> f`.
    EmptyStage,
    /// После `<`, `>` или `>>` нет имени файла.
    MissingTarget,
    /// `&` не в конце строки.
    AmpersandNotLast,
}
// ANCHOR_END: types

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ParseError::Unterminated => "кавычка без пары",
            ParseError::EmptyStage => "пустая команда",
            ParseError::MissingTarget => "нет имени файла для перенаправления",
            ParseError::AmpersandNotLast => "& — только в конце строки",
        })
    }
}

// ANCHOR: parse
enum Tok {
    Word(String),
    Pipe,
    In,
    Out { append: bool },
    Amp,
}

/// Слова и операторы. Кавычки склеивают слово: `'a b'` и `"a b"` — одно
/// слово; в двойных кавычках `\"` и `\\` — сами символы.
fn tokens(line: &str) -> Result<Vec<Tok>, ParseError> {
    let mut toks = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        let op = match c {
            c if c.is_whitespace() => None,
            '|' => Some(Tok::Pipe),
            '<' => Some(Tok::In),
            '&' => Some(Tok::Amp),
            '>' => {
                chars.next();
                let append = chars.next_if_eq(&'>').is_some();
                toks.push(Tok::Out { append });
                continue;
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || "|<>&".contains(c) {
                        break;
                    }
                    chars.next();
                    match c {
                        '\'' => loop {
                            match chars.next().ok_or(ParseError::Unterminated)? {
                                '\'' => break,
                                c => word.push(c),
                            }
                        },
                        '"' => loop {
                            match chars.next().ok_or(ParseError::Unterminated)? {
                                '"' => break,
                                '\\' => match chars.next().ok_or(ParseError::Unterminated)? {
                                    c @ ('"' | '\\') => word.push(c),
                                    c => word.extend(['\\', c]),
                                },
                                c => word.push(c),
                            }
                        },
                        c => word.push(c),
                    }
                }
                toks.push(Tok::Word(word));
                continue;
            }
        };
        chars.next();
        toks.extend(op);
    }
    Ok(toks)
}

/// Разобрать строку. Пустая строка — `Ok(None)`.
pub fn parse(line: &str) -> Result<Option<Job>, ParseError> {
    let toks = tokens(line)?;
    if toks.is_empty() {
        return Ok(None);
    }
    let mut stages = Vec::new();
    let mut stage = Stage::default();
    let mut background = false;
    let mut it = toks.into_iter();
    while let Some(tok) = it.next() {
        if background {
            return Err(ParseError::AmpersandNotLast);
        }
        match tok {
            Tok::Word(w) => stage.argv.push(w),
            Tok::In => match it.next() {
                Some(Tok::Word(path)) => stage.input = Some(path),
                _ => return Err(ParseError::MissingTarget),
            },
            Tok::Out { append } => match it.next() {
                Some(Tok::Word(path)) => stage.output = Some(Redirect { path, append }),
                _ => return Err(ParseError::MissingTarget),
            },
            Tok::Pipe if stage.argv.is_empty() => return Err(ParseError::EmptyStage),
            Tok::Pipe => stages.push(std::mem::take(&mut stage)),
            Tok::Amp => background = true,
        }
    }
    if stage.argv.is_empty() {
        return Err(ParseError::EmptyStage);
    }
    stages.push(stage);
    Ok(Some(Job { stages, background }))
}
// ANCHOR_END: parse

// ANCHOR: child
/// Сигналы управления заданиями, которые оболочка игнорирует. SIG_IGN
/// переживает execve — поэтому ребёнок возвращает их по умолчанию сам,
/// иначе Ctrl-C не прервал бы и задание. SIGTSTP здесь нет: без `fg`
/// остановленное задание не продолжить, и задания игнорируют его, как
/// сама оболочка (`take_terminal`).
const JOB_SIGNALS: [i32; 4] = [libc::SIGINT, libc::SIGQUIT, libc::SIGTTIN, libc::SIGTTOU];

/// Код ребёнка между fork и exec. Здесь можно только async-signal-safe:
/// tcsetpgrp и signal — из этого списка, а rustix делает их прямыми
/// системными вызовами, без аллокаций и блокировок.
fn child_setup(tty: Option<RawFd>) -> impl FnMut() -> io::Result<()> + Send + Sync + 'static {
    move || {
        if let Some(fd) = tty {
            // Задание переднего плана берёт терминал и само: иначе оно
            // может прочитать его раньше, чем это сделает оболочка, и
            // остановиться по SIGTTIN. SIGTTOU пока ещё игнорируется.
            // SAFETY: дескриптор терминала открыт у оболочки, а ребёнок
            // унаследовал копию.
            let tty = unsafe { BorrowedFd::borrow_raw(fd) };
            tcsetpgrp(tty, getpgrp())?;
        }
        for sig in JOB_SIGNALS {
            // SAFETY: signal async-signal-safe, обработчик — SIG_DFL.
            unsafe { libc::signal(sig, libc::SIG_DFL) };
        }
        Ok(())
    }
}
// ANCHOR_END: child

// ANCHOR: spawn
/// Запущенное задание.
pub struct Running {
    /// Группа процессов: pid первой запущенной команды.
    pub pgid: Option<Pid>,
    pub children: Vec<Child>,
    /// Последнюю команду запустить не удалось: статус задания — 127.
    pub last_failed: bool,
}

fn raw(pid: Pid) -> i32 {
    pid.as_raw_nonzero().get()
}

fn open_output(r: &Redirect) -> io::Result<File> {
    File::options().write(true).create(true).append(r.append).truncate(!r.append).open(&r.path)
}

/// Запустить стадии задания, соединив их каналами. `tty` — терминал, если
/// оболочка интерактивна и задание — переднего плана.
pub fn spawn(job: &Job, tty: Option<BorrowedFd<'_>>) -> Running {
    let mut running = Running { pgid: None, children: Vec::new(), last_failed: false };
    let mut prev: Option<OwnedFd> = None;
    let n = job.stages.len();
    for (i, stage) in job.stages.iter().enumerate() {
        let mut cmd = Command::new(&stage.argv[0]);
        cmd.args(&stage.argv[1..]);
        // Канал к следующей стадии создаётся всегда: если вывод
        // перенаправлен в файл, следующая стадия прочитает пустой канал.
        let (next, to_next) = match i + 1 < n {
            true => match pipe_with(PipeFlags::CLOEXEC) {
                Ok((r, w)) => (Some(r), Some(w)),
                Err(e) => {
                    eprintln!("ch28-shell: pipe: {e}");
                    break;
                }
            },
            false => (None, None),
        };
        let from_prev = prev.take();
        let setup = || -> io::Result<()> {
            if let Some(path) = &stage.input {
                cmd.stdin(File::open(path)?);
            } else if let Some(r) = from_prev {
                cmd.stdin(Stdio::from(r));
            }
            if let Some(out) = &stage.output {
                cmd.stdout(open_output(out)?);
            } else if let Some(w) = to_next {
                cmd.stdout(Stdio::from(w));
            }
            Ok(())
        };
        let result = setup().and_then(|()| {
            // Вся стадия — в группу первой; 0 — новая группа с pgid = pid.
            cmd.process_group(running.pgid.map_or(0, raw));
            // SAFETY: замыкание делает только async-signal-safe вызовы.
            unsafe { cmd.pre_exec(child_setup(tty.map(|t| t.as_raw_fd()))) };
            cmd.spawn()
        });
        // Здесь cmd уничтожается и закрывает копии концов каналов у
        // оболочки: иначе читатель не дождался бы конца потока.
        drop(cmd);
        match result {
            Ok(child) => {
                let pid = Pid::from_child(&child);
                if running.pgid.is_none() {
                    running.pgid = Some(pid);
                    if let Some(tty) = tty {
                        // То же, что в ребёнке: кто первый — неважно.
                        tcsetpgrp(tty, pid).ok();
                    }
                }
                running.children.push(child);
                running.last_failed = false;
            }
            Err(e) => {
                let what = if e.kind() == io::ErrorKind::NotFound {
                    "команда не найдена".to_owned()
                } else {
                    e.to_string()
                };
                eprintln!("ch28-shell: {}: {what}", stage.argv[0]);
                running.last_failed = true;
            }
        }
        prev = next;
    }
    running
}
// ANCHOR_END: spawn

// ANCHOR: status
/// Код задания, как у sh: код выхода или 128 + номер сигнала.
pub fn code(status: ExitStatus) -> i32 {
    status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
}

fn describe(status: ExitStatus) -> Option<String> {
    if let Some(sig) = status.signal() {
        let name = nix::sys::signal::Signal::try_from(sig).map_or("?", |s| s.as_str());
        return Some(format!("сигнал {name}"));
    }
    status.code().filter(|&c| c != 0).map(|c| format!("код {c}"))
}

/// Дождаться всех стадий. Статус задания — статус последней.
pub fn wait(running: &mut Running) -> i32 {
    let mut last = None;
    for child in &mut running.children {
        last = child.wait().ok();
    }
    if running.last_failed {
        eprintln!("ch28-shell: [код 127]");
        return 127;
    }
    match last {
        Some(status) => {
            if let Some(text) = describe(status) {
                eprintln!("ch28-shell: [{text}]");
            }
            code(status)
        }
        None => 127,
    }
}
// ANCHOR_END: status

// ANCHOR: repl
/// Интерактивный режим: stdin — терминал. Оболочка — в своей группе и
/// на переднем плане, сигналы управления заданиями ей не страшны.
fn take_terminal() -> io::Result<Option<OwnedFd>> {
    let stdin = rustix::stdio::stdin();
    if !isatty(stdin) {
        return Ok(None);
    }
    for sig in JOB_SIGNALS.into_iter().chain([libc::SIGTSTP]) {
        // SAFETY: SIG_IGN — никакого кода в обработчике.
        unsafe { libc::signal(sig, libc::SIG_IGN) };
    }
    if getpgrp() != getpid() {
        setpgid(None, None)?;
    }
    // Своя копия терминала: stdin детей заменяют каналы и файлы.
    let tty = rustix::io::fcntl_dupfd_cloexec(stdin, 10)?;
    tcsetpgrp(&tty, getpgrp())?;
    Ok(Some(tty))
}

/// Цикл оболочки. Возвращает код выхода: аргумент `exit` или код
/// последнего задания.
pub fn repl() -> i32 {
    let tty = match take_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("ch28-shell: терминал: {e}");
            return 1;
        }
    };
    let mut background: Vec<Running> = Vec::new();
    let mut status = 0;
    let mut line = String::new();
    loop {
        // Фоновые задания, которые кончились, — до подсказки, как в sh.
        background.retain_mut(|job| {
            let done = job.children.iter_mut().all(|c| matches!(c.try_wait(), Ok(Some(_))));
            if done {
                eprintln!("[{}] готово", job.pgid.map_or(0, raw));
            }
            !done
        });
        if tty.is_some() {
            eprint!("$ ");
        }
        line.clear();
        match io::stdin().lock().read_line(&mut line) {
            Ok(0) => return status,
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                eprintln!("ch28-shell: stdin: {e}");
                return 1;
            }
        }
        let job = match parse(&line) {
            Ok(Some(job)) => job,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("ch28-shell: {e}");
                status = 2;
                continue;
            }
        };
        let argv = &job.stages[0].argv;
        match argv[0].as_str() {
            // Встроенные команды меняют саму оболочку, поэтому не могут
            // быть отдельными программами.
            "exit" if job.stages.len() == 1 => {
                return argv.get(1).and_then(|c| c.parse().ok()).unwrap_or(status);
            }
            "cd" if job.stages.len() == 1 => {
                let dir = argv
                    .get(1)
                    .cloned()
                    .or_else(|| std::env::var("HOME").ok())
                    .unwrap_or_else(|| "/".into());
                status = match std::env::set_current_dir(&dir) {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("ch28-shell: cd: {dir}: {e}");
                        1
                    }
                };
                continue;
            }
            _ => {}
        }
        if job.background {
            let running = spawn(&job, None);
            eprintln!("[{}]", running.pgid.map_or(0, raw));
            background.push(running);
            status = 0;
        } else {
            let mut running = spawn(&job, tty.as_ref().map(|t| t.as_fd()));
            status = wait(&mut running);
            if let Some(tty) = &tty {
                // Терминал — обратно оболочке. Она теперь в фоновой группе,
                // и без игнорирования SIGTTOU ядро остановило бы её здесь.
                tcsetpgrp(tty, getpgrp()).ok();
            }
        }
    }
}
// ANCHOR_END: repl

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses() {
        assert_eq!(code(ExitStatus::from_raw(0)), 0);
        assert_eq!(code(ExitStatus::from_raw(3 << 8)), 3);
        assert_eq!(code(ExitStatus::from_raw(libc::SIGINT)), 130);
        assert_eq!(describe(ExitStatus::from_raw(libc::SIGINT)).as_deref(), Some("сигнал SIGINT"));
        assert_eq!(describe(ExitStatus::from_raw(0)), None);
    }
}
