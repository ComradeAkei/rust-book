//! Шаг главы 13 (задача П13): итератор по кадрам в потоке байт.
//! ЗАГОТОВКА: типы, поля и конструкторы даны, тела `next`, `size_hint`,
//! `rest` и `stats` — `todo!()`. Проверка: `cargo test -p project-frame
//! p13`.
//!
//! [`Frames`] идёт по буферу и отдаёт кадры с их смещениями, а ошибки — с
//! местом, как [`RxError`]. После ошибки, при которой заголовку верить
//! нельзя, итератор ищет следующий маркер [`MAGIC`] и продолжает с него,
//! поэтому участок мусора даёт одну ошибку, а кадры после него не теряются.
//! Неполный кадр в конце буфера итератор оставляет: поток может
//! продолжиться, и [`Frames::rest`] отдаёт хвост, который надо дополнить
//! новыми байтами. Когда продолжения не будет (сработал тайм-аут приёма),
//! буфер разбирают через [`Frames::until_eof`], и неполный кадр становится
//! ошибкой.

// Импорты и поля заготовки пока не используются, и это ожидаемо.
#![allow(unused_imports, dead_code)]

use crate::{Frame, MAGIC, Recovery, RxError, decode_at};
use core::iter::FusedIterator;

// ANCHOR: frames
/// Кадры буфера по порядку: `Ok((смещение, кадр))` или ошибка с местом.
#[derive(Clone, Debug)]
pub struct Frames<'a> {
    buf: &'a [u8],
    /// Смещение первого неразобранного байта, не больше `buf.len()`.
    pos: usize,
    /// Продолжения потока не будет: неполный кадр — ошибка.
    eof: bool,
}

impl<'a> Frames<'a> {
    /// Поток продолжается: на неполном кадре в конце буфера итератор
    /// останавливается.
    pub const fn new(buf: &'a [u8]) -> Self {
        Frames { buf, pos: 0, eof: false }
    }

    /// Поток закончился: неполный кадр — ошибка, после неё итератор ищет
    /// следующий маркер.
    pub const fn until_eof(buf: &'a [u8]) -> Self {
        Frames { buf, pos: 0, eof: true }
    }

    /// Смещение первого неразобранного байта.
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Неразобранный хвост буфера.
    pub fn rest(&self) -> &'a [u8] {
        todo!()
    }
}

impl<'a> Iterator for Frames<'a> {
    type Item = Result<(usize, Frame<'a>), RxError>;

    /// Кадр — `Ok((смещение, кадр))` и шаг за кадр. Ошибка — `Err` и
    /// поиск следующего маркера со следующего байта, кроме неполного
    /// кадра в потоке, который продолжается: тогда `None`, и `position`
    /// остаётся на начале неполного кадра.
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }

    /// Каждый элемент забирает хотя бы один байт, а элементов может не
    /// быть вовсе: неполный кадр в начале.
    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

/// Обещание: после первого `None` — только `None`. Его проверяет тест
/// `p13_fused_and_size_hint`.
impl FusedIterator for Frames<'_> {}
// ANCHOR_END: frames

// ANCHOR: stats
/// Сводка по закончившемуся потоку.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Stats {
    /// Кадров каждого типа: измерения, события, подтверждения.
    pub frames: [usize; 3],
    pub errors: usize,
    /// Байтов вне целых кадров.
    pub junk: usize,
}

/// Через `Frames::until_eof` и свёртку.
pub fn stats(buf: &[u8]) -> Stats {
    todo!()
}
// ANCHOR_END: stats
