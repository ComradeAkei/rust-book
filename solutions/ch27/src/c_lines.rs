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

    // ANCHOR: feed
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
        match byte {
            b'\r' | b'\n' => {
                echo.put(b"\r\n");
                let (n, overflow) = (self.len, self.overflow);
                (self.len, self.overflow) = (0, false);
                if overflow {
                    Feed::TooLong
                } else if n == 0 {
                    Feed::Pending
                } else {
                    // В буфере только ASCII: ошибки разбора UTF-8 быть не может.
                    Feed::Line(core::str::from_utf8(&self.buf[..n]).unwrap_or_default())
                }
            }
            0x08 | 0x7F => {
                if self.len > 0 && !self.overflow {
                    self.len -= 1;
                    echo.put(b"\x08 \x08");
                }
                Feed::Pending
            }
            0x20..=0x7E => {
                if self.len < N && !self.overflow {
                    self.buf[self.len] = byte;
                    self.len += 1;
                    echo.put(&[byte]);
                } else {
                    if !self.overflow {
                        echo.put(b"\x07");
                    }
                    self.overflow = true;
                }
                Feed::Pending
            }
            _ => Feed::Pending,
        }
    }
    // ANCHOR_END: feed
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

// ANCHOR: number
/// Десятичное число только из цифр: `str::parse::<u32>` принял бы и `+5`.
fn number(s: &str) -> Result<u32, CmdError> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CmdError::BadNumber);
    }
    s.parse().map_err(|_| CmdError::BadNumber)
}
// ANCHOR_END: number

// ANCHOR: parse
/// Команда из строки: слова разделены пробелами, регистр важен.
pub fn parse(line: &str) -> Result<Command, CmdError> {
    let mut words = line.split_ascii_whitespace();
    let cmd = match words.next().ok_or(CmdError::Empty)? {
        "status" => Command::Status,
        "start" => Command::Start,
        "stop" => Command::Stop,
        "period" => {
            let ms = number(words.next().ok_or(CmdError::MissingArg)?)?;
            if !PERIOD_MS.contains(&ms) {
                return Err(CmdError::OutOfRange);
            }
            Command::Period(ms)
        }
        _ => return Err(CmdError::Unknown),
    };
    match words.next() {
        Some(_) => Err(CmdError::Extra),
        None => Ok(cmd),
    }
}
// ANCHOR_END: parse
