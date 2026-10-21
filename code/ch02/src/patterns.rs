//! Раздел 2.3: `match` и образцы.
//!
//! `match` обязан покрыть все значения. Пропущенный вариант — ошибка
//! компиляции, а не предупреждение:
//!
//! ```compile_fail,E0004
//! enum Mode {
//!     Idle,
//!     Rx,
//!     Tx,
//!     Sleep, // добавлен позже, чем написана mode_name
//! }
//!
//! fn mode_name(m: Mode) -> &'static str {
//!     match m {
//!         Mode::Idle => "idle",
//!         Mode::Rx => "rx",
//!         Mode::Tx => "tx",
//!     }
//! }
//! ```
//!
//! Имя в образце, которое не нашлось среди констант, — новая переменная,
//! и такая ветка ловит всё подряд. Компилятор предупреждает, а с
//! `deny` — отказывается собирать:
//!
//! ```compile_fail
//! #![deny(unreachable_patterns)]
//! mod proto {
//!     pub const ACK: u8 = 0x06;
//!     pub const NAK: u8 = 0x15;
//! }
//!
//! pub fn reply(b: u8) -> &'static str {
//!     match b {
//!         ACK => "ack", // забыли `use proto::*`: это переменная
//!         NAK => "nak",
//!         _ => "?",
//!     }
//! }
//! ```

// ANCHOR: classify
/// Класс байта во входном потоке.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Digit(u8),
    Letter(u8),
    Space,
    Control,
    NonAscii,
    Other,
}

/// Ветки проверяются сверху вниз: `\t`, `\r` и `\n` попадают в `Space`,
/// хотя лежат и в диапазоне управляющих символов.
pub fn classify(b: u8) -> Class {
    match b {
        b'0'..=b'9' => Class::Digit(b - b'0'),
        c @ (b'a'..=b'z' | b'A'..=b'Z') => Class::Letter(c.to_ascii_lowercase()),
        b' ' | b'\t' | b'\r' | b'\n' => Class::Space,
        0..0x20 | 0x7F => Class::Control,
        0x80.. => Class::NonAscii,
        _ => Class::Other,
    }
}
// ANCHOR_END: classify

// ANCHOR: consts
/// Байты подтверждения из протокола. Константа в образце сравнивается по
/// значению.
pub const ACK: u8 = 0x06;
pub const NAK: u8 = 0x15;

pub fn reply(b: u8) -> Option<bool> {
    match b {
        ACK => Some(true),
        NAK => Some(false),
        _ => None,
    }
}
// ANCHOR_END: consts

// ANCHOR: slices
/// Начало кадра.
pub const START: u8 = 0x7E;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    Empty,
    NoStart,
    Truncated,
    BadLength,
    BadChecksum,
}

/// Кадр: `START`, команда, длина, данные, сумма байтов команды, длины и
/// данных по модулю 256. Возвращает команду и данные.
pub fn parse_frame(buf: &[u8]) -> Result<(u8, &[u8]), FrameError> {
    match buf {
        [] => Err(FrameError::Empty),
        [START, cmd, len, payload @ .., sum] => {
            if usize::from(*len) != payload.len() {
                return Err(FrameError::BadLength);
            }
            let body = &buf[1..buf.len() - 1];
            let actual = body.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
            if actual != *sum {
                return Err(FrameError::BadChecksum);
            }
            Ok((*cmd, payload))
        }
        [START, ..] => Err(FrameError::Truncated),
        [_, ..] => Err(FrameError::NoStart),
    }
}
// ANCHOR_END: slices

// ANCHOR: first_last
/// Первый и последний элементы. `match *xs` сопоставляет сам срез, а не
/// ссылку на него, и `first`, `last` получаются копиями `u32`.
pub fn first_last(xs: &[u32]) -> Option<(u32, u32)> {
    match *xs {
        [] => None,
        [only] => Some((only, only)),
        [first, .., last] => Some((first, last)),
    }
}
// ANCHOR_END: first_last

// ANCHOR: state_table
/// Состояние соединения с базовой станцией.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Down,
    Connecting { tries: u8 },
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    PowerOn,
    Ack,
    Timeout,
    Fault,
}

/// Переход автомата. `match` по паре — таблица состояний: строка на каждый
/// значимый переход, остальное ловят последние ветки.
pub fn step(state: Link, event: Event) -> Link {
    match (state, event) {
        (Link::Down, Event::PowerOn) => Link::Connecting { tries: 1 },
        (Link::Connecting { .. }, Event::Ack) => Link::Up,
        (Link::Connecting { tries }, Event::Timeout) if tries < 3 => {
            Link::Connecting { tries: tries + 1 }
        }
        (Link::Connecting { .. }, Event::Timeout) => Link::Down,
        (Link::Up, Event::Timeout) => Link::Connecting { tries: 1 },
        (_, Event::Fault) => Link::Down,
        (s, _) => s,
    }
}
// ANCHOR_END: state_table

// ANCHOR: binding_modes
/// Счётчики ошибок по каналам. Образец без `&` сопоставляется со ссылкой
/// `&mut Option<u32>`, и `n` получается `&mut u32`: режим связывания
/// `ref mut` выбран неявно.
pub fn bump(counter: &mut Option<u32>) {
    match counter {
        Some(n) => *n += 1,
        None => *counter = Some(1),
    }
}

/// То же с явными `ref mut` и `&mut`: так писали до 2018 года.
pub fn bump_explicit(counter: &mut Option<u32>) {
    match *counter {
        Some(ref mut n) => *n += 1,
        None => *counter = Some(1),
    }
}
// ANCHOR_END: binding_modes

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes() {
        assert_eq!(classify(b'7'), Class::Digit(7));
        assert_eq!(classify(b'Q'), Class::Letter(b'q'));
        assert_eq!(classify(b'\n'), Class::Space);
        assert_eq!(classify(0x1B), Class::Control);
        assert_eq!(classify(0x7F), Class::Control);
        assert_eq!(classify(0xD0), Class::NonAscii);
        assert_eq!(classify(b'+'), Class::Other);
    }

    #[test]
    fn consts() {
        assert_eq!(reply(0x06), Some(true));
        assert_eq!(reply(0x15), Some(false));
        assert_eq!(reply(0x00), None);
    }

    #[test]
    fn frames() {
        // 0x10 + 2 + 0xAA + 0x55 = 0x111, по модулю 256 — 0x11.
        let ok = [START, 0x10, 2, 0xAA, 0x55, 0x11];
        assert_eq!(parse_frame(&ok), Ok((0x10, &ok[3..5])));
        assert_eq!(parse_frame(&[START, 0x01, 0, 0x01]), Ok((0x01, &[][..])));
        assert_eq!(parse_frame(&[]), Err(FrameError::Empty));
        assert_eq!(parse_frame(&[0x00, 1, 2, 3]), Err(FrameError::NoStart));
        assert_eq!(parse_frame(&[START, 1, 0]), Err(FrameError::Truncated));
        assert_eq!(parse_frame(&[START, 1, 5, 0]), Err(FrameError::BadLength));
        assert_eq!(parse_frame(&[START, 1, 0, 0]), Err(FrameError::BadChecksum));
    }

    #[test]
    fn ends() {
        assert_eq!(first_last(&[]), None);
        assert_eq!(first_last(&[5]), Some((5, 5)));
        assert_eq!(first_last(&[1, 2, 3]), Some((1, 3)));
    }

    #[test]
    fn link() {
        let mut s = Link::Down;
        for e in [Event::PowerOn, Event::Timeout, Event::Timeout] {
            s = step(s, e);
        }
        assert_eq!(s, Link::Connecting { tries: 3 });
        assert_eq!(step(s, Event::Timeout), Link::Down);
        assert_eq!(step(s, Event::Ack), Link::Up);
        assert_eq!(step(Link::Up, Event::PowerOn), Link::Up);
        assert_eq!(step(Link::Up, Event::Fault), Link::Down);
    }

    #[test]
    fn binding_modes() {
        let mut c = None;
        bump(&mut c);
        bump(&mut c);
        bump_explicit(&mut c);
        assert_eq!(c, Some(3));
    }
}
