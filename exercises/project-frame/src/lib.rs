//! Сквозной проект «станция телеметрии», шаг главы 6: формат кадра.
//! ЗАГОТОВКА: реализуйте функции, помеченные `todo!()`, по спецификации
//! ниже и проверьте себя тестами `cargo test -p project-frame`.
//!
//! Крейт `no_std` и без аллокаций: его подключают и прошивка на Cortex-M,
//! и хост на Linux. Кадр на проводе (все многобайтовые поля little-endian):
//!
//! ```text
//! смещение  размер  поле
//! 0         1       magic   = 0xA5
//! 1         1       version = 1
//! 2         1       kind    (1 измерение, 2 событие, 3 подтверждение)
//! 3         1       seq     номер кадра по модулю 256
//! 4         2       len     длина полезной нагрузки, не больше 240
//! 6         2       crc     CRC-16/CCITT-FALSE по байтам 0..6 и нагрузке
//! 8         len     payload
//! ```
#![no_std]
// Параметры заготовок пока не используются, и это ожидаемо.
#![allow(unused_variables)]

use core::mem::{align_of, offset_of, size_of};
use core::num::NonZeroU16;

pub const MAGIC: u8 = 0xA5;
pub const VERSION: u8 = 1;
pub const HEADER_LEN: usize = 8;
pub const MAX_PAYLOAD: usize = 240;

// ANCHOR: wire
/// Образ заголовка на проводе. Все поля имеют выравнивание 1, поэтому
/// структуру можно наложить на буфер по любому адресу, а порядок байт
/// многобайтовых полей задан явно, а не процессором.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HeaderWire {
    pub magic: u8,
    pub version: u8,
    pub kind: u8,
    pub seq: u8,
    pub len: [u8; 2],
    pub crc: [u8; 2],
}

const _: () = {
    assert!(size_of::<HeaderWire>() == HEADER_LEN);
    assert!(align_of::<HeaderWire>() == 1);
    assert!(offset_of!(HeaderWire, len) == 4);
    assert!(offset_of!(HeaderWire, crc) == 6);
};
// ANCHOR_END: wire

// ANCHOR: kind
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Kind {
    Measurement = 1,
    Event = 2,
    Ack = 3,
}

impl TryFrom<u8> for Kind {
    type Error = FrameError;
    fn try_from(b: u8) -> Result<Self, FrameError> {
        todo!()
    }
}
// ANCHOR_END: kind

// ANCHOR: error
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrameError {
    /// Во входном буфере меньше байт, чем нужно для кадра.
    Short {
        need: usize,
        have: usize,
    },
    /// В выходном буфере не хватает места.
    BufferTooSmall {
        need: usize,
        have: usize,
    },
    BadMagic(u8),
    BadVersion(u8),
    BadKind(u8),
    TooLong(usize),
    BadCrc {
        stored: u16,
        computed: u16,
    },
    /// Нагрузка не соответствует своему типу (длина, нулевой датчик).
    BadPayload,
}
// ANCHOR_END: error

// ANCHOR: crc
/// Пошаговое обновление CRC-16/CCITT-FALSE: полином 0x1021, начальное
/// значение 0xFFFF, без отражения битов. Контрольное значение для
/// "123456789" равно 0x29B1.
pub const fn crc16_update(crc: u16, data: &[u8]) -> u16 {
    todo!()
}

pub const fn crc16(data: &[u8]) -> u16 {
    crc16_update(0xFFFF, data)
}

// ANCHOR_END: crc

// ANCHOR: frame
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Header {
    pub kind: Kind,
    pub seq: u8,
    pub len: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Frame<'a> {
    pub header: Header,
    pub payload: &'a [u8],
}

impl Frame<'_> {
    /// Сколько байт кадр занимает на проводе: столько надо пропустить,
    /// чтобы перейти к следующему кадру в потоке.
    pub const fn wire_len(&self) -> usize {
        HEADER_LEN + self.payload.len()
    }
}
// ANCHOR_END: frame

// ANCHOR: encode
/// Записывает кадр в `out` и возвращает число записанных байт.
pub fn encode(kind: Kind, seq: u8, payload: &[u8], out: &mut [u8]) -> Result<usize, FrameError> {
    todo!()
}
// ANCHOR_END: encode

// ANCHOR: decode
/// Разбирает кадр в начале `buf`. Байты после кадра не трогает.
pub fn decode(buf: &[u8]) -> Result<Frame<'_>, FrameError> {
    todo!()
}
// ANCHOR_END: decode

// ANCHOR: measurement
/// Нагрузка кадра `Kind::Measurement`. Датчик с номером 0 не существует,
/// поэтому `Option<Measurement>` не больше самого `Measurement`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Measurement {
    pub sensor: NonZeroU16,
    pub timestamp_ms: u32,
    pub value: i32,
}

impl Measurement {
    pub const WIRE_LEN: usize = 10;

    pub fn to_bytes(&self) -> [u8; Self::WIRE_LEN] {
        todo!()
    }

    pub fn from_bytes(payload: &[u8]) -> Result<Self, FrameError> {
        todo!()
    }
}

const _: () = assert!(size_of::<Option<Measurement>>() == size_of::<Measurement>());
// ANCHOR_END: measurement
