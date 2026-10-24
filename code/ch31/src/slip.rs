//! Раздел 31.3: байт-стаффинг SLIP (RFC 1055) — кадры по UART без поля
//! длины. Конец кадра — байт `END`; `END` и `ESC` внутри данных
//! заменяются парами `ESC ESC_END` и `ESC ESC_ESC`.

use core::fmt;

pub const END: u8 = 0xC0;
pub const ESC: u8 = 0xDB;
pub const ESC_END: u8 = 0xDC;
pub const ESC_ESC: u8 = 0xDD;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlipError {
    /// Кадр не кончается байтом `END`.
    Unterminated,
    /// `END` посреди кадра.
    EndInside(usize),
    /// После `ESC` стоит не `ESC_END` и не `ESC_ESC`.
    BadEscape { at: usize, byte: u8 },
}

impl fmt::Display for SlipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SlipError::Unterminated => f.write_str("кадр без END"),
            SlipError::EndInside(at) => write!(f, "END внутри кадра на смещении {at}"),
            SlipError::BadEscape { at, byte } => {
                write!(f, "после ESC на смещении {at} байт {byte:#04x}")
            }
        }
    }
}

impl std::error::Error for SlipError {}

// ANCHOR: v1
/// Первая версия: экранирует `END`, а `ESC` в данных пишет как есть.
pub fn encode_v1(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 1);
    for &b in data {
        match b {
            END => out.extend_from_slice(&[ESC, ESC_END]),
            b => out.push(b),
        }
    }
    out.push(END);
    out
}
// ANCHOR_END: v1

// ANCHOR: encode
pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 1);
    for &b in data {
        match b {
            END => out.extend_from_slice(&[ESC, ESC_END]),
            ESC => out.extend_from_slice(&[ESC, ESC_ESC]),
            b => out.push(b),
        }
    }
    out.push(END);
    out
}
// ANCHOR_END: encode

// ANCHOR: decode
/// Данные кадра `wire`, который кончается `END`.
pub fn decode(wire: &[u8]) -> Result<Vec<u8>, SlipError> {
    let Some((&END, body)) = wire.split_last() else {
        return Err(SlipError::Unterminated);
    };
    let mut out = Vec::with_capacity(body.len());
    let mut it = body.iter().copied().enumerate();
    while let Some((at, b)) = it.next() {
        match b {
            END => return Err(SlipError::EndInside(at)),
            ESC => match it.next() {
                Some((_, ESC_END)) => out.push(END),
                Some((_, ESC_ESC)) => out.push(ESC),
                Some((at, byte)) => return Err(SlipError::BadEscape { at, byte }),
                // ESC последним байтом тела: за ним сразу END кадра.
                None => return Err(SlipError::BadEscape { at: at + 1, byte: END }),
            },
            b => out.push(b),
        }
    }
    Ok(out)
}
// ANCHOR_END: decode

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc1055_examples() {
        assert_eq!(encode(&[1, END, 2, ESC, 3]), [1, ESC, ESC_END, 2, ESC, ESC_ESC, 3, END]);
        assert_eq!(
            decode(&[1, ESC, ESC_END, 2, ESC, ESC_ESC, 3, END]),
            Ok(vec![1, END, 2, ESC, 3])
        );
        assert_eq!(decode(&[END]), Ok(vec![]));
        assert_eq!(decode(&[1, 2]), Err(SlipError::Unterminated));
        assert_eq!(decode(&[1, END, 2, END]), Err(SlipError::EndInside(1)));
        assert_eq!(decode(&[ESC, 7, END]), Err(SlipError::BadEscape { at: 1, byte: 7 }));
    }

    /// Ошибку первой версии пример из документации не ловит: в нём нет ESC.
    #[test]
    fn v1_passes_the_obvious_test() {
        assert_eq!(decode(&encode_v1(b"hello")), Ok(b"hello".to_vec()));
        assert_eq!(decode(&encode_v1(&[END])), Ok(vec![END]));
        assert_eq!(decode(&encode_v1(&[ESC])), Err(SlipError::BadEscape { at: 1, byte: END }));
    }
}
