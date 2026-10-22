//! Задача B: иерархия ошибок загрузки конфигурации станции поверх лексера
//! из задачи 5.B. Три уровня: ввод-вывод, синтаксис с позицией, семантика;
//! контекст — путь к файлу и номер строки; цепочка `source` без сторонних
//! крейтов.

use ch05_solutions::b_lexer::{LexError, LexErrorKind, Lexer, Token};
use std::error::Error;
use std::fmt;
use std::io;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};

/// Скорости UART, которые поддерживает станция.
pub const BAUDS: [u32; 5] = [9600, 19200, 38400, 57600, 115200];
/// Скорость, если в секции `[uart]` нет ключа `baud` или нет самой секции.
pub const DEFAULT_BAUD: u32 = 115200;
/// Допустимый период опроса датчика, мс.
pub const PERIOD_MS: core::ops::RangeInclusive<u32> = 10..=60_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StationConfig {
    pub baud: u32,
    /// Датчики в порядке появления в файле.
    pub sensors: Vec<Sensor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sensor {
    pub id: u16,
    pub period_ms: u32,
}

// ANCHOR: types
/// Ошибка лексера главы 5. `LexError` — чужой тип, `Display` для него в
/// этом крейте реализовать нельзя (правило сироты), поэтому нужна обёртка.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxError(pub LexError);

#[derive(Debug)]
pub enum SemanticError {
    KeyOutsideSection(String),
    UnknownSection(String),
    UnknownKey { section: String, key: String },
    BadNumber { key: String, source: ParseIntError },
    OutOfRange { key: String, value: u32 },
    BadBaud(u32),
    DuplicateSensor(u16),
    MissingKey { section: String, key: &'static str },
}

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Syntax(SyntaxError),
    /// Вариант в куче: `SemanticError` с двумя строками занимает 48 байт.
    Semantic {
        line: u32,
        source: Box<SemanticError>,
    },
    /// Рекурсивный вариант обязан быть в куче.
    InFile {
        path: PathBuf,
        source: Box<ConfigError>,
    },
}
// ANCHOR_END: types

// ANCHOR: display
impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.0.kind {
            LexErrorKind::UnclosedSection => "нет закрывающей ]",
            LexErrorKind::BadName => "недопустимое имя",
            LexErrorKind::MissingEquals => "нет знака =",
            LexErrorKind::UnclosedQuote => "нет закрывающей кавычки",
            LexErrorKind::TrailingGarbage => "лишние символы в конце строки",
        };
        write!(f, "строка {}, столбец {}: {what}", self.0.line, self.0.col)
    }
}

impl Error for SyntaxError {}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SemanticError::*;
        match self {
            KeyOutsideSection(key) => write!(f, "ключ {key} вне секции"),
            UnknownSection(s) => write!(f, "неизвестная секция [{s}]"),
            UnknownKey { section, key } => write!(f, "неизвестный ключ {key} в секции [{section}]"),
            BadNumber { key, .. } => write!(f, "значение {key} не число"),
            OutOfRange { key, value } => write!(f, "{key} = {value} вне допустимого диапазона"),
            BadBaud(b) => write!(f, "скорость {b} не поддерживается"),
            DuplicateSensor(id) => write!(f, "датчик {id} описан дважды"),
            MissingKey { section, key } => write!(f, "в секции [{section}] нет ключа {key}"),
        }
    }
}

impl Error for SemanticError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SemanticError::BadNumber { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io { path, .. } => write!(f, "не удалось прочитать {}", path.display()),
            ConfigError::Syntax(_) => f.write_str("синтаксическая ошибка"),
            ConfigError::Semantic { line, .. } => write!(f, "ошибка в строке {line}"),
            ConfigError::InFile { path, .. } => {
                write!(f, "{}: конфигурация отвергнута", path.display())
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Io { source, .. } => Some(source),
            ConfigError::Syntax(e) => Some(e),
            ConfigError::Semantic { source, .. } => Some(&**source),
            ConfigError::InFile { source, .. } => Some(&**source),
        }
    }
}
// ANCHOR_END: display

impl From<LexError> for ConfigError {
    fn from(e: LexError) -> Self {
        ConfigError::Syntax(SyntaxError(e))
    }
}

// ANCHOR: line
/// Номер строки лексемы. Лексемы главы 5 — срезы исходного текста, поэтому
/// позиция восстанавливается по адресу начала среза.
fn line_of(src: &str, token: &str) -> u32 {
    let offset = token.as_ptr() as usize - src.as_ptr() as usize;
    src.as_bytes()[..offset].iter().filter(|&&b| b == b'\n').count() as u32 + 1
}
// ANCHOR_END: line

enum Section {
    None,
    Uart,
    Sensor { id: u16, header_line: u32, period_ms: Option<u32> },
}

fn semantic(line: u32, e: SemanticError) -> ConfigError {
    ConfigError::Semantic { line, source: Box::new(e) }
}

fn number(key: &str, value: &str) -> Result<u32, SemanticError> {
    value.parse().map_err(|source| SemanticError::BadNumber { key: key.to_string(), source })
}

/// Закрыть секцию датчика: проверить обязательные ключи.
fn close(section: Section, out: &mut StationConfig) -> Result<(), ConfigError> {
    if let Section::Sensor { id, header_line, period_ms } = section {
        let Some(period_ms) = period_ms else {
            let section = format!("sensor.{id}");
            return Err(semantic(
                header_line,
                SemanticError::MissingKey { section, key: "period_ms" },
            ));
        };
        out.sensors.push(Sensor { id, period_ms });
    }
    Ok(())
}

fn section_name(s: &Section) -> String {
    match s {
        Section::None => String::new(),
        Section::Uart => "uart".to_string(),
        Section::Sensor { id, .. } => format!("sensor.{id}"),
    }
}

// ANCHOR: parse
pub fn parse(src: &str) -> Result<StationConfig, ConfigError> {
    let mut out = StationConfig { baud: DEFAULT_BAUD, sensors: Vec::new() };
    let mut section = Section::None;
    let mut key: Option<&str> = None;
    for token in Lexer::new(src) {
        match token? {
            Token::Section(name) => {
                let line = line_of(src, name);
                close(core::mem::replace(&mut section, Section::None), &mut out)?;
                section = match name.strip_prefix("sensor.") {
                    None if name == "uart" => Section::Uart,
                    Some(n) => match n.parse::<u16>() {
                        Ok(id) if id != 0 => {
                            if out.sensors.iter().any(|s| s.id == id) {
                                return Err(semantic(line, SemanticError::DuplicateSensor(id)));
                            }
                            Section::Sensor { id, header_line: line, period_ms: None }
                        }
                        _ => {
                            return Err(semantic(line, SemanticError::UnknownSection(name.into())));
                        }
                    },
                    None => return Err(semantic(line, SemanticError::UnknownSection(name.into()))),
                };
            }
            Token::Key(k) => key = Some(k),
            Token::Value(v) => {
                let k = key.take().expect("лексер выдаёт Value только после Key");
                let line = line_of(src, k);
                let fail = |e| semantic(line, e);
                match (&mut section, k) {
                    (Section::None, _) => {
                        return Err(fail(SemanticError::KeyOutsideSection(k.into())));
                    }
                    (Section::Uart, "baud") => {
                        let b = number(k, v).map_err(fail)?;
                        if !BAUDS.contains(&b) {
                            return Err(fail(SemanticError::BadBaud(b)));
                        }
                        out.baud = b;
                    }
                    (Section::Sensor { period_ms, .. }, "period_ms") => {
                        let p = number(k, v).map_err(fail)?;
                        if !PERIOD_MS.contains(&p) {
                            let e = SemanticError::OutOfRange { key: k.into(), value: p };
                            return Err(fail(e));
                        }
                        *period_ms = Some(p);
                    }
                    (s, _) => {
                        let e =
                            SemanticError::UnknownKey { section: section_name(s), key: k.into() };
                        return Err(fail(e));
                    }
                }
            }
        }
    }
    close(section, &mut out)?;
    Ok(out)
}
// ANCHOR_END: parse

// ANCHOR: load
pub fn load(path: &Path) -> Result<StationConfig, ConfigError> {
    let text = std::fs::read_to_string(path)
        .map_err(|source| ConfigError::Io { path: path.to_path_buf(), source })?;
    parse(&text).map_err(|e| ConfigError::InFile { path: path.to_path_buf(), source: Box::new(e) })
}
// ANCHOR_END: load

/// Отчёт: сообщение и все причины, как `report` из раздела 7.3.
pub fn report(e: &(dyn Error + 'static)) -> String {
    let mut out = format!("ошибка: {e}\n");
    let mut cur = e.source();
    while let Some(c) = cur {
        out += &format!("  причина: {c}\n");
        cur = c.source();
    }
    out
}
