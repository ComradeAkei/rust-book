//! Задача C: команды станции по UART. Байты приходят по одному из
//! обработчика прерывания приёма, редактор собирает из них строку — с эхом
//! и забоем, как в терминале, — а разбор превращает строку в команду.
//! UART здесь не нужен: эхо уходит в любой [`Sink`], и всё проверяется на
//! хосте. Прошивка задачи — `firmware/ch27-tasks`, программа `c_uart`.

use core::ops::RangeInclusive;

// ANCHOR: sink
/// Куда редактор пишет эхо. Прошивка отдаёт сюда UART, тест — буфер.
pub trait Sink {
    fn put(&mut self, bytes: &[u8]);
}
// ANCHOR_END: sink

// ANCHOR: editor
/// Что получилось после очередного байта.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feed<'a> {
    /// Строка ещё не закончена (или закончилась пустой).
    Pending,
    /// Готовая строка без `\r` и `\n`.
    Line(&'a str),
    /// Строка не поместилась в буфер и отброшена целиком.
    TooLong,
}

/// Редактор строки на `N` байт без кучи.
pub struct LineEditor<const N: usize> {
    buf: [u8; N],
    len: usize,
    overflow: bool,
}
// ANCHOR_END: editor

impl<const N: usize> Default for LineEditor<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> LineEditor<N> {
    pub const fn new() -> Self {
        LineEditor { buf: [0; N], len: 0, overflow: false }
    }

    /// Один байт с UART.
    ///
    /// - `\r` или `\n` заканчивает строку, эхо — `\r\n`; пустая строка —
    ///   `Pending`, так что пара `\r\n` даёт одну строку, а не две.
    /// - Забой (0x08) и DEL (0x7F) стирают последний символ, эхо —
    ///   `\x08 \x08`; в пустой строке они ничего не делают.
    /// - Печатные символы ASCII (0x20..=0x7E) копятся с эхом. Символ, не
    ///   поместившийся в буфер, вместо эха даёт звонок 0x07, и вся строка
    ///   до конца отбрасывается: `TooLong`.
    /// - Остальные байты молча пропускаются.
    pub fn feed(&mut self, byte: u8, echo: &mut impl Sink) -> Feed<'_> {
        let _ = (byte, echo, &mut self.buf, &mut self.len, &mut self.overflow);
        todo!("задача C: feed")
    }
}

// ANCHOR: command
/// Допустимый период выборки, мс.
pub const PERIOD_MS: RangeInclusive<u32> = 10..=10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Status,
    Start,
    Stop,
    Period(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdError {
    /// Пустая строка или одни пробелы.
    Empty,
    Unknown,
    /// `period` без числа.
    MissingArg,
    /// Не десятичное число без знака или больше u32::MAX.
    BadNumber,
    /// Число вне [`PERIOD_MS`].
    OutOfRange,
    /// Лишние слова после команды.
    Extra,
}
// ANCHOR_END: command

impl CmdError {
    pub const fn message(self) -> &'static str {
        match self {
            CmdError::Empty => "пустая команда",
            CmdError::Unknown => "неизвестная команда",
            CmdError::MissingArg => "нужно число",
            CmdError::BadNumber => "не число",
            CmdError::OutOfRange => "период вне 10..10000 мс",
            CmdError::Extra => "лишние слова",
        }
    }
}

/// Команда из строки: слова разделены пробелами, регистр важен. Число —
/// только десятичные цифры: `str::parse::<u32>` принял бы и `+5`.
pub fn parse(line: &str) -> Result<Command, CmdError> {
    let _ = line;
    todo!("задача C: parse")
}
