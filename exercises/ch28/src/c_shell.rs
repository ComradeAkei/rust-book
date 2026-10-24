//! Задача C: мини-оболочка. Конвейеры `a | b | c`, перенаправления `<`,
//! `>`, `>>`, фоновые задания `&` и встроенные `cd` и `exit`. Каждое
//! задание — своя группа процессов; в интерактивном режиме оболочка
//! отдаёт терминал группе задания, и Ctrl-C прерывает только его.
//!
//! Сообщения, которые проверяют тесты, — в stderr:
//!
//! - `ch28-shell: [код N]` — последняя команда задания вышла с кодом N ≠ 0;
//! - `ch28-shell: [сигнал SIGINT]` — её убил сигнал (имя — как в nix);
//! - `ch28-shell: имя: команда не найдена` — программы нет, код 127;
//! - `ch28-shell: <ParseError>` — строку не разобрать, код 2;
//! - `[pgid]` — фоновое задание запущено, `[pgid] готово` — кончилось;
//! - `$ ` — подсказка, только в интерактивном режиме.
//!
//! Код выхода оболочки — аргумент `exit` или код последней строки.

use std::fmt;

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

// ANCHOR: api
/// Разобрать строку. Пустая строка — `Ok(None)`. Слова разделены
/// пробелами; `'a b'` и `"a b"` — одно слово, в двойных кавычках `\"` и
/// `\\` — сами символы. Операторы `|`, `<`, `>`, `>>`, `&` пробелов
/// вокруг не требуют: `cat<in>>log`.
pub fn parse(line: &str) -> Result<Option<Job>, ParseError> {
    let _ = line;
    todo!("задача C: parse")
}

/// Цикл оболочки: читать строки stdin, запускать задания, ждать задания
/// переднего плана. Если stdin — терминал: своя группа процессов,
/// SIGINT, SIGQUIT, SIGTSTP, SIGTTIN и SIGTTOU игнорируются, терминал
/// отдаётся группе задания на время его работы.
pub fn repl() -> i32 {
    todo!("задача C: repl")
}
// ANCHOR_END: api
