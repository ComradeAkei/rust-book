//! Шаг главы 12 (задача П12): разбор кадра комбинаторами на замыканиях.
//! ЗАГОТОВКА: типы и сигнатуры даны, тела — `todo!()` внутри замыканий,
//! потому что `impl Parser` должен быть замыканием, а не `!`. Проверка:
//! `cargo test -p project-frame p12`.
//!
//! Парсер — функция из байтов в «значение и остаток» либо ошибку кадра.
//! `decode` отсюда должен давать на любом входе то же, что
//! [`crate::decode`], а LLVM — встроить весь парсер в одну функцию без
//! косвенных вызовов и без вызовов других функций этого модуля.
// Импорты и вспомогательные функции заготовки пока не используются, и это
// ожидаемо.
#![allow(unused_imports, dead_code)]

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
    move |i: &'a [u8]| todo!()
}

pub fn byte<'a>() -> impl Parser<'a, u8> {
    |i: &'a [u8]| todo!()
}

/// Байт, равный `want`; другой байт `err` превращает в ошибку.
/// Конструктор варианта, например `FrameError::BadMagic`, — такая функция.
pub fn expect<'a>(want: u8, err: impl Fn(u8) -> FrameError) -> impl Parser<'a, u8> {
    move |i: &'a [u8]| todo!()
}

pub fn le_u16<'a>() -> impl Parser<'a, u16> {
    |i: &'a [u8]| todo!()
}

pub fn le_u32<'a>() -> impl Parser<'a, u32> {
    |i: &'a [u8]| todo!()
}
// ANCHOR_END: primitives

// ANCHOR: combinators
pub fn map<'a, A, B>(p: impl Parser<'a, A>, f: impl Fn(A) -> B) -> impl Parser<'a, B> {
    move |i| todo!()
}

/// Как `map`, но `f` может отвергнуть значение своей ошибкой.
pub fn try_map<'a, A, B>(
    p: impl Parser<'a, A>,
    f: impl Fn(A) -> Result<B, FrameError>,
) -> impl Parser<'a, B> {
    move |i| todo!()
}

/// `p`, затем парсер, который `f` строит по результату `p`: длина нагрузки
/// известна только из заголовка.
pub fn then<'a, A, B, Q: Parser<'a, B>>(
    p: impl Parser<'a, A>,
    f: impl Fn(&A) -> Q,
) -> impl Parser<'a, (A, B)> {
    move |i| todo!()
}
// ANCHOR_END: combinators

// ANCHOR: frame
/// Поля заголовка по порядку. Внутри восьми байт заголовка нехватки не
/// бывает, и последовательность удобнее записать замыканием с `?`, чем
/// вложенными парами.
fn fields<'a>() -> impl Parser<'a, (Header, u16)> {
    |i: &'a [u8]| todo!()
}

/// Заголовок: сырые восемь байт, по которым считается CRC, и поля.
fn header<'a>() -> impl Parser<'a, (&'a [u8], Header, u16)> {
    |i: &'a [u8]| todo!()
}

/// Кадр целиком: заголовок, нагрузка его длины, проверка CRC.
pub fn frame<'a>() -> impl Parser<'a, Frame<'a>> {
    |i: &'a [u8]| todo!()
}
// ANCHOR_END: frame

// ANCHOR: decode
/// То же, что [`crate::decode`]. `Short` от `take` считается от места, где
/// не хватило байт, а кадру нужен счёт от его начала: до этого места
/// прочитано `buf.len() - have` байт.
pub fn decode(buf: &[u8]) -> Result<Frame<'_>, FrameError> {
    todo!()
}
// ANCHOR_END: decode

// ANCHOR: measurement
/// Нагрузка измерения: номер датчика (не ноль), время и значение.
pub fn measurement<'a>() -> impl Parser<'a, Measurement> {
    |i: &'a [u8]| todo!()
}

/// То же, что [`Measurement::from_bytes`]: любая неудача, в том числе
/// лишние или недостающие байты, — `BadPayload`.
pub fn measurement_from(payload: &[u8]) -> Result<Measurement, FrameError> {
    todo!()
}
// ANCHOR_END: measurement
