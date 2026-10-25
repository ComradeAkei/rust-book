//! Хранилище измерений в памяти хоста.
//!
//! У каждого датчика — кольцо [`VecDeque`] из последних `cap` измерений.
//! Новое измерение вытесняет самое старое, пока кольцо полно, и буфер
//! кольца при этом не переезжает: место выделено один раз при появлении
//! датчика. Запрос по диапазону времени находит границы двоичным поиском
//! ([`slice::partition_point`] на кольце), раз измерения одного датчика
//! приходят по возрастанию времени.

use core::num::NonZeroU16;
use std::collections::HashMap;
use std::collections::VecDeque;
use telemetry_frame::stream::Frames;
use telemetry_frame::{Kind, Measurement};

// ANCHOR: point
/// Измерение без номера датчика: номер — это ключ в хранилище, и в кольце
/// его дублировать незачем. Точка — 8 байт: меньше и `Measurement` в
/// памяти (12), и кадра на проводе (18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub t_ms: u32,
    pub value: i32,
}
// ANCHOR_END: point

// ANCHOR: ring
/// Кольцо последних `cap` измерений одного датчика.
#[derive(Debug)]
struct Ring {
    buf: VecDeque<Point>,
    cap: usize,
}

impl Ring {
    fn with_cap(cap: usize) -> Self {
        // with_capacity выделяет буфер сразу, целиком. Дальше push вытесняет
        // старое на том же буфере: ни одного realloc в установившемся режиме.
        Ring { buf: VecDeque::with_capacity(cap.max(1)), cap: cap.max(1) }
    }

    fn push(&mut self, p: Point) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(p);
    }
}
// ANCHOR_END: ring

// ANCHOR: store
/// Измерения всех датчиков: по кольцу на датчик.
#[derive(Debug)]
pub struct Store {
    sensors: HashMap<NonZeroU16, Ring>,
    cap: usize,
}
// ANCHOR_END: store

// ANCHOR: ingested
/// Итог разбора одной порции байт: сколько кадров каждого типа встретилось
/// (измерения, события, подтверждения), сколько точек легло в хранилище,
/// сколько измерений отвергнуто по нагрузке и сколько ошибок разбора потока.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ingested {
    pub frames: [usize; 3],
    pub stored: usize,
    pub bad_payload: usize,
    pub errors: usize,
}
// ANCHOR_END: ingested

// ANCHOR: summary
/// Сводка по хранилищу: число датчиков, всего точек и границы времени.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub sensors: usize,
    pub points: usize,
    pub oldest_ms: Option<u32>,
    pub newest_ms: Option<u32>,
}
// ANCHOR_END: summary

impl Store {
    /// Хранилище, где у каждого датчика хранятся последние `cap` измерений.
    pub fn new(cap: usize) -> Self {
        Store { sensors: HashMap::new(), cap: cap.max(1) }
    }

    /// Сколько измерений хранится у одного датчика.
    pub const fn capacity(&self) -> usize {
        self.cap
    }

    /// Сколько датчиков встречено.
    pub fn sensor_count(&self) -> usize {
        self.sensors.len()
    }

    // ANCHOR: ingest
    /// Разобрать поток байт как кадры (до конца, по правилам главы 13) и
    /// сложить измерения в кольца. Возвращает, что встретилось.
    ///
    /// В установившемся режиме — те же датчики, кольца полны — не делает ни
    /// одного обращения к распределителю: `Frames` работает без кучи,
    /// `entry` по known-ключу ничего не выделяет, а `push` в полном кольце
    /// только двигает голову и хвост.
    pub fn ingest(&mut self, bytes: &[u8]) -> Ingested {
        let mut rep = Ingested::default();
        for item in Frames::until_eof(bytes) {
            match item {
                Ok((_, frame)) => {
                    rep.frames[frame.header.kind as usize - 1] += 1;
                    if frame.header.kind == Kind::Measurement {
                        match Measurement::from_bytes(frame.payload) {
                            Ok(m) => {
                                let cap = self.cap;
                                self.sensors
                                    .entry(m.sensor)
                                    .or_insert_with(|| Ring::with_cap(cap))
                                    .push(Point { t_ms: m.timestamp_ms, value: m.value });
                                rep.stored += 1;
                            }
                            Err(_) => rep.bad_payload += 1,
                        }
                    }
                }
                Err(_) => rep.errors += 1,
            }
        }
        rep
    }
    // ANCHOR_END: ingest

    // ANCHOR: range
    /// Измерения датчика с временем в `[from_ms, to_ms]`. Границы находятся
    /// двоичным поиском: измерения в кольце упорядочены по времени, и
    /// `partition_point` делит его на «раньше границы» и «не раньше».
    pub fn range(
        &self,
        sensor: NonZeroU16,
        from_ms: u32,
        to_ms: u32,
    ) -> impl Iterator<Item = &Point> {
        self.sensors.get(&sensor).into_iter().flat_map(move |r| {
            let lo = r.buf.partition_point(|p| p.t_ms < from_ms);
            let hi = r.buf.partition_point(|p| p.t_ms <= to_ms);
            r.buf.range(lo..hi)
        })
    }
    // ANCHOR_END: range

    /// Самое свежее измерение датчика.
    pub fn latest(&self, sensor: NonZeroU16) -> Option<&Point> {
        self.sensors.get(&sensor)?.buf.back()
    }

    /// Сколько измерений хранится у датчика сейчас.
    pub fn len(&self, sensor: NonZeroU16) -> usize {
        self.sensors.get(&sensor).map_or(0, |r| r.buf.len())
    }

    // ANCHOR: sorted
    /// Номера датчиков по возрастанию. Это отчёт для человека, а не
    /// горячий путь: порядок обхода `HashMap` случаен, поэтому список
    /// собирается в `Vec` и сортируется — одно выделение на вызов.
    pub fn sensors_sorted(&self) -> Vec<NonZeroU16> {
        let mut v: Vec<NonZeroU16> = self.sensors.keys().copied().collect();
        v.sort_unstable();
        v
    }
    // ANCHOR_END: sorted

    // ANCHOR: summary_fn
    /// Сводка по всему хранилищу: обход колец без единого выделения.
    pub fn summary(&self) -> Summary {
        let mut s = Summary { sensors: self.sensors.len(), ..Summary::default() };
        for r in self.sensors.values() {
            s.points += r.buf.len();
            if let Some(f) = r.buf.front() {
                s.oldest_ms = Some(s.oldest_ms.map_or(f.t_ms, |o| o.min(f.t_ms)));
            }
            if let Some(b) = r.buf.back() {
                s.newest_ms = Some(s.newest_ms.map_or(b.t_ms, |n| n.max(b.t_ms)));
            }
        }
        s
    }
    // ANCHOR_END: summary_fn
}
