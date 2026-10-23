//! ЗАГОТОВКА хранилища. Типы и конструкторы даны, тела методов — `todo!()`.
//!
//! Условия:
//! - у каждого датчика — кольцо из последних `cap` измерений; новое
//!   вытесняет самое старое, и буфер кольца не должен переезжать, когда
//!   кольцо уже полно (подсказка: `VecDeque::with_capacity`);
//! - [`Store::ingest`] разбирает байты как кадры через
//!   `telemetry_frame::stream::Frames::until_eof` и складывает измерения;
//!   в установившемся режиме не должен обращаться к куче;
//! - [`Store::range`] находит границы окна времени двоичным поиском
//!   (`partition_point` на кольце), границы включаются;
//! - [`Store::sensors_sorted`] возвращает номера по возрастанию.

#![allow(unused_imports, unused_variables, dead_code)]

use core::num::NonZeroU16;
use std::collections::HashMap;
use std::collections::VecDeque;
use telemetry_frame::stream::Frames;
use telemetry_frame::{Kind, Measurement};

/// Измерение без номера датчика: номер — это ключ в хранилище.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub t_ms: u32,
    pub value: i32,
}

/// Кольцо последних `cap` измерений одного датчика.
#[derive(Debug)]
struct Ring {
    buf: VecDeque<Point>,
    cap: usize,
}

impl Ring {
    fn with_cap(cap: usize) -> Self {
        Ring { buf: VecDeque::with_capacity(cap.max(1)), cap: cap.max(1) }
    }

    fn push(&mut self, p: Point) {
        todo!()
    }
}

#[derive(Debug)]
pub struct Store {
    sensors: HashMap<NonZeroU16, Ring>,
    cap: usize,
}

/// Итог разбора одной порции байт.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ingested {
    pub frames: [usize; 3],
    pub stored: usize,
    pub bad_payload: usize,
    pub errors: usize,
}

/// Сводка по хранилищу.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub sensors: usize,
    pub points: usize,
    pub oldest_ms: Option<u32>,
    pub newest_ms: Option<u32>,
}

impl Store {
    pub fn new(cap: usize) -> Self {
        Store { sensors: HashMap::new(), cap: cap.max(1) }
    }

    pub const fn capacity(&self) -> usize {
        self.cap
    }

    pub fn sensor_count(&self) -> usize {
        self.sensors.len()
    }

    /// Разобрать поток байт как кадры и сложить измерения в кольца.
    pub fn ingest(&mut self, bytes: &[u8]) -> Ingested {
        todo!()
    }

    /// Измерения датчика с временем в `[from_ms, to_ms]`.
    pub fn range(
        &self,
        sensor: NonZeroU16,
        from_ms: u32,
        to_ms: u32,
    ) -> impl Iterator<Item = &Point> {
        let _ = (from_ms, to_ms);
        todo!();
        // Недостижимо: задаёт компилятору форму возвращаемого итератора.
        // Замените `0..0` настоящими границами окна из `partition_point`.
        #[allow(unreachable_code)]
        self.sensors.get(&sensor).into_iter().flat_map(|r| r.buf.range(0..0))
    }

    pub fn latest(&self, sensor: NonZeroU16) -> Option<&Point> {
        todo!()
    }

    pub fn len(&self, sensor: NonZeroU16) -> usize {
        todo!()
    }

    /// Номера датчиков по возрастанию.
    pub fn sensors_sorted(&self) -> Vec<NonZeroU16> {
        todo!()
    }

    /// Сводка по всему хранилищу.
    pub fn summary(&self) -> Summary {
        todo!()
    }
}
