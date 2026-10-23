//! Шаг главы 12: разбор кадра комбинаторами на замыканиях.
//!
//! Парсер — функция из байтов в «значение и остаток» либо ошибку кадра.
//! Комбинатор берёт парсеры и возвращает новый — замыкание, которое
//! захватило их по значению. `decode` отсюда даёт на любом входе то же,
//! что [`crate::decode`], а LLVM встраивает весь парсер в одну функцию
//! без косвенных вызовов.

use crate::{
    Frame, FrameError, HEADER_LEN, Header, Kind, MAGIC, MAX_PAYLOAD, Measurement, VERSION,
};
use crate::{crc16, crc16_update};
use core::num::NonZeroU16;

// ANCHOR: parser
/// Значение и непрочитанный остаток либо ошибка.
pub type PResult<'a, T> = Result<(T, &'a [u8]), FrameError>;

/// Короткое имя для «функции из `&[u8]` в `PResult`».
pub trait Parser<'a, T>: Fn(&'a [u8]) -> PResult<'a, T> {}

impl<'a, T, F: Fn(&'a [u8]) -> PResult<'a, T>> Parser<'a, T> for F {}
// ANCHOR_END: parser

// ANCHOR: primitives
/// Ровно `n` байт. Нехватка — `Short` в счёте от этого места: сколько
/// нужно и сколько осталось здесь.
pub fn take<'a>(n: usize) -> impl Parser<'a, &'a [u8]> {
    move |i: &'a [u8]| match i.split_at_checked(n) {
        Some(parts) => Ok(parts),
        None => Err(FrameError::Short { need: n, have: i.len() }),
    }
}

pub fn byte<'a>() -> impl Parser<'a, u8> {
    |i: &'a [u8]| match i.split_first() {
        Some((&b, rest)) => Ok((b, rest)),
        None => Err(FrameError::Short { need: 1, have: 0 }),
    }
}

/// Байт, равный `want`; другой байт `err` превращает в ошибку.
/// Конструктор варианта, например `FrameError::BadMagic`, — такая функция.
pub fn expect<'a>(want: u8, err: impl Fn(u8) -> FrameError) -> impl Parser<'a, u8> {
    let b = byte();
    move |i| {
        let (got, rest) = b(i)?;
        if got == want { Ok((got, rest)) } else { Err(err(got)) }
    }
}

pub fn le_u16<'a>() -> impl Parser<'a, u16> {
    map(take(2), |b: &[u8]| u16::from_le_bytes([b[0], b[1]]))
}

pub fn le_u32<'a>() -> impl Parser<'a, u32> {
    map(take(4), |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
// ANCHOR_END: primitives

// ANCHOR: combinators
pub fn map<'a, A, B>(p: impl Parser<'a, A>, f: impl Fn(A) -> B) -> impl Parser<'a, B> {
    move |i| p(i).map(|(a, rest)| (f(a), rest))
}

/// Как `map`, но `f` может отвергнуть значение своей ошибкой.
pub fn try_map<'a, A, B>(
    p: impl Parser<'a, A>,
    f: impl Fn(A) -> Result<B, FrameError>,
) -> impl Parser<'a, B> {
    move |i| {
        let (a, rest) = p(i)?;
        Ok((f(a)?, rest))
    }
}

/// `p`, затем парсер, который `f` строит по результату `p`: длина нагрузки
/// известна только из заголовка.
pub fn then<'a, A, B, Q: Parser<'a, B>>(
    p: impl Parser<'a, A>,
    f: impl Fn(&A) -> Q,
) -> impl Parser<'a, (A, B)> {
    move |i| {
        let (a, rest) = p(i)?;
        let (b, rest) = f(&a)(rest)?;
        Ok(((a, b), rest))
    }
}
// ANCHOR_END: combinators

// ANCHOR: frame
/// Поля заголовка по порядку. Внутри восьми байт заголовка нехватки не
/// бывает, и последовательность удобнее записать замыканием с `?`, чем
/// вложенными парами.
fn fields<'a>() -> impl Parser<'a, (Header, u16)> {
    let magic = expect(MAGIC, FrameError::BadMagic);
    let version = expect(VERSION, FrameError::BadVersion);
    let kind = try_map(byte(), Kind::try_from);
    let len = try_map(le_u16(), |n| {
        if usize::from(n) > MAX_PAYLOAD { Err(FrameError::TooLong(n.into())) } else { Ok(n) }
    });
    move |i| {
        let (_, i) = magic(i)?;
        let (_, i) = version(i)?;
        let (kind, i) = kind(i)?;
        let (seq, i) = byte()(i)?;
        let (len, i) = len(i)?;
        let (crc, i) = le_u16()(i)?;
        Ok(((Header { kind, seq, len }, crc), i))
    }
}

/// Заголовок: сырые восемь байт, по которым считается CRC, и поля.
fn header<'a>() -> impl Parser<'a, (&'a [u8], Header, u16)> {
    try_map(take(HEADER_LEN), |raw| {
        let ((header, crc), _) = fields()(raw)?;
        Ok((raw, header, crc))
    })
}

/// Кадр целиком: заголовок, нагрузка его длины, проверка CRC.
pub fn frame<'a>() -> impl Parser<'a, Frame<'a>> {
    let with_payload = then(header(), |&(_, h, _)| take(usize::from(h.len)));
    try_map(with_payload, |((raw, header, stored), payload)| {
        let computed = crc16_update(crc16(&raw[..6]), payload);
        if stored != computed {
            return Err(FrameError::BadCrc { stored, computed });
        }
        Ok(Frame { header, payload })
    })
}
// ANCHOR_END: frame

// ANCHOR: decode
/// То же, что [`crate::decode`]. `Short` от `take` считается от места, где
/// не хватило байт, а кадру нужен счёт от его начала: до этого места
/// прочитано `buf.len() - have` байт.
pub fn decode(buf: &[u8]) -> Result<Frame<'_>, FrameError> {
    match frame()(buf) {
        Ok((f, _)) => Ok(f),
        Err(FrameError::Short { need, have }) => {
            Err(FrameError::Short { need: buf.len() - have + need, have: buf.len() })
        }
        Err(e) => Err(e),
    }
}
// ANCHOR_END: decode

// ANCHOR: measurement
/// Нагрузка измерения: номер датчика (не ноль), время и значение.
pub fn measurement<'a>() -> impl Parser<'a, Measurement> {
    let sensor = try_map(le_u16(), |n| NonZeroU16::new(n).ok_or(FrameError::BadPayload));
    let value = map(le_u32(), |v| v as i32);
    move |i| {
        let (sensor, i) = sensor(i)?;
        let (timestamp_ms, i) = le_u32()(i)?;
        let (value, i) = value(i)?;
        Ok((Measurement { sensor, timestamp_ms, value }, i))
    }
}

/// То же, что [`Measurement::from_bytes`]: любая неудача, в том числе
/// лишние или недостающие байты, — `BadPayload`.
pub fn measurement_from(payload: &[u8]) -> Result<Measurement, FrameError> {
    match measurement()(payload) {
        Ok((m, [])) => Ok(m),
        _ => Err(FrameError::BadPayload),
    }
}
// ANCHOR_END: measurement
