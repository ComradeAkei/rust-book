//! Шаг главы 13: итератор по кадрам в потоке байт.
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
        &self.buf[self.pos..]
    }

    /// Встать на следующий маркер начиная с `from` или на конец буфера.
    fn resync(&mut self, from: usize) {
        let tail = &self.buf[from..];
        self.pos = tail.iter().position(|&b| b == MAGIC).map_or(self.buf.len(), |i| from + i);
    }
}

impl<'a> Iterator for Frames<'a> {
    type Item = Result<(usize, Frame<'a>), RxError>;

    fn next(&mut self) -> Option<Self::Item> {
        let at = self.pos;
        if at == self.buf.len() {
            return None;
        }
        match decode_at(self.buf, at) {
            Ok(frame) => {
                self.pos = at + frame.wire_len();
                Some(Ok((at, frame)))
            }
            Err(e) => match e.error.recovery() {
                Recovery::NeedMore(_) if !self.eof => None,
                // Испорченный заголовок или неполный кадр, которому уже не
                // дождаться продолжения. Drop и Reject decode не выдаёт, но
                // и для них безопаснее искать маркер, чем верить длине.
                Recovery::NeedMore(_) | Recovery::Resync | Recovery::Drop | Recovery::Reject => {
                    self.resync(at + 1);
                    Some(Err(e))
                }
            },
        }
    }

    /// Каждый элемент забирает хотя бы один байт, а элементов может не
    /// быть вовсе: неполный кадр в начале.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.buf.len() - self.pos))
    }
}

/// Буфер не меняется, поэтому на неполном кадре или в конце итератор
/// отвечает `None` и дальше.
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

pub fn stats(buf: &[u8]) -> Stats {
    let (mut s, in_frames) =
        Frames::until_eof(buf).fold((Stats::default(), 0), |(mut s, n), item| match item {
            Ok((_, f)) => {
                s.frames[f.header.kind as usize - 1] += 1;
                (s, n + f.wire_len())
            }
            Err(_) => {
                s.errors += 1;
                (s, n)
            }
        });
    s.junk = buf.len() - in_frames;
    s
}
// ANCHOR_END: stats
