//! Раздел 7.3: трейт `Error`, цепочка `source` и контекст без сторонних
//! крейтов. Всё здесь работает в `no_std`: трейт живёт в `core::error`.

use alloc::string::String;
use core::error::Error;
use core::fmt::{self, Write};
use core::num::ParseIntError;

// ANCHOR: leaf
/// Ошибка нижнего уровня: позиция в тексте конфигурации.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "синтаксическая ошибка в строке {}, столбце {}", self.line, self.col)
    }
}

impl Error for SyntaxError {}
// ANCHOR_END: leaf

// ANCHOR: middle
/// Ошибка разбора конфигурации. Каждый вариант хранит причину и отдаёт её
/// через `source`, но в своё сообщение её текст не включает.
#[derive(Debug)]
pub enum ConfigError {
    Syntax(SyntaxError),
    BadNumber { key: &'static str, source: ParseIntError },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Syntax(_) => f.write_str("не удалось разобрать конфигурацию"),
            ConfigError::BadNumber { key, .. } => write!(f, "ключ {key}: ожидалось число"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Syntax(e) => Some(e),
            ConfigError::BadNumber { source, .. } => Some(source),
        }
    }
}
// ANCHOR_END: middle

// ANCHOR: context
/// Контекст: что делала программа, когда случилась ошибка `E`.
#[derive(Debug)]
pub struct Context<E> {
    pub what: String,
    pub source: E,
}

impl<E> fmt::Display for Context<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.what)
    }
}

impl<E: Error + 'static> Error for Context<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Обернуть ошибку контекстом. Строка строится только на пути ошибки.
pub fn context<T, E>(r: Result<T, E>, what: impl FnOnce() -> String) -> Result<T, Context<E>> {
    r.map_err(|source| Context { what: what(), source })
}
// ANCHOR_END: context

// ANCHOR: report
/// Все звенья цепочки, начиная с самой ошибки.
pub fn chain<'a>(e: &'a (dyn Error + 'static)) -> impl Iterator<Item = &'a (dyn Error + 'static)> {
    core::iter::successors(Some(e), |&e| e.source())
}

/// Отчёт для человека: сообщение и все причины по одной на строку.
pub fn report(e: &(dyn Error + 'static)) -> String {
    let mut out = String::new();
    for (i, link) in chain(e).enumerate() {
        let lead = if i == 0 { "ошибка" } else { "  причина" };
        let _ = writeln!(out, "{lead}: {link}");
    }
    out
}
// ANCHOR_END: report

// ANCHOR: usage
pub fn parse_rate(line: u32, text: &str) -> Result<u32, Context<ConfigError>> {
    let r = match text.split_once('=') {
        None => Err(ConfigError::Syntax(SyntaxError { line, col: 1 })),
        Some((_, v)) => {
            v.trim().parse().map_err(|source| ConfigError::BadNumber { key: "rate", source })
        }
    };
    context(r, || alloc::format!("чтение station.ini, строка {line}"))
}
// ANCHOR_END: usage

// ANCHOR: twice
/// Так делать не надо: сообщение повторяет текст причины, и отчёт по
/// цепочке печатает его дважды.
#[derive(Debug)]
pub struct Chatty(pub ParseIntError);

impl fmt::Display for Chatty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "плохое число: {}", self.0)
    }
}

impl Error for Chatty {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
// ANCHOR_END: twice

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_walks_the_chain() {
        let e = parse_rate(3, "rate = 1x0").unwrap_err();
        assert_eq!(
            report(&e),
            "ошибка: чтение station.ini, строка 3\n  \
             причина: ключ rate: ожидалось число\n  \
             причина: invalid digit found in string\n"
        );
        let e = parse_rate(5, "rate 100").unwrap_err();
        assert_eq!(chain(&e).count(), 3);
        let last = chain(&e).last().unwrap();
        assert_eq!(last.downcast_ref::<SyntaxError>(), Some(&SyntaxError { line: 5, col: 1 }));
        assert_eq!(parse_rate(1, "rate = 100").unwrap(), 100);
    }

    #[test]
    fn chatty_repeats_itself() {
        let e = Chatty("x".parse::<u8>().unwrap_err());
        assert_eq!(
            report(&e),
            "ошибка: плохое число: invalid digit found in string\n  \
             причина: invalid digit found in string\n"
        );
    }
}
