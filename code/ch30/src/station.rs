//! Раздел 30.9: приём станции под профилировщиком — поток кадров и
//! хранилище из проекта.

use core::hash::{BuildHasherDefault, Hasher};
use core::num::NonZeroU16;
use std::collections::{HashMap, VecDeque};
use telemetry_frame::stream::Frames;
use telemetry_frame::{
    CRC16_TABLE, Frame, HEADER_LEN, Header, Kind, MAGIC, MAX_PAYLOAD, Measurement, VERSION,
    crc16_update_table, encode,
};
use telemetry_store::{Ingested, Point, Summary};

// ANCHOR: stream
/// Поток из `frames` кадров: измерения `sensors` датчиков по кругу, время
/// растёт на 10 мс за круг; каждый шестнадцатый кадр — событие на
/// четыре байта.
pub fn stream(frames: usize, sensors: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(frames * 18);
    let mut buf = [0u8; 32];
    let mut m = 0usize;
    for i in 0..frames {
        let seq = i as u8;
        let n = if i % 16 == 15 {
            encode(Kind::Event, seq, &(i as u32).to_le_bytes(), &mut buf)
        } else {
            let s = (m % usize::from(sensors)) as u16;
            let point = Measurement {
                sensor: NonZeroU16::new(100 + 7 * s).unwrap(),
                timestamp_ms: (m / usize::from(sensors)) as u32 * 10,
                value: (m as i32 % 2000) - 1000,
            };
            m += 1;
            encode(Kind::Measurement, seq, &point.to_bytes(), &mut buf)
        };
        out.extend_from_slice(&buf[..n.unwrap()]);
    }
    out
}
// ANCHOR_END: stream

// ANCHOR: crc_x2
/// Вторая таблица CRC-16/CCITT-FALSE: `CRC16_X2[h]` — два шага по
/// таблице для регистра `h << 8`. Вместе с `CRC16_TABLE` она обновляет
/// CRC сразу на два байта.
pub static CRC16_X2: [u16; 256] = {
    let mut t = [0u16; 256];
    let mut h = 0;
    while h < 256 {
        let one = CRC16_TABLE[h];
        t[h] = (one << 8) ^ CRC16_TABLE[(one >> 8) as usize];
        h += 1;
    }
    t
};

/// CRC по два байта за шаг. Регистр после двух байт — линейная функция
/// от `crc ^ (b0 << 8 | b1)`: старший байт даёт `CRC16_X2`, младший —
/// `CRC16_TABLE`. Два чтения таблиц не зависят друг от друга, и цепочка
/// зависимостей вдвое короче, чем у побайтового цикла.
pub fn crc16_update_x2(mut crc: u16, data: &[u8]) -> u16 {
    let (pairs, tail) = data.as_chunks::<2>();
    for &[b0, b1] in pairs {
        let x = crc ^ u16::from_be_bytes([b0, b1]);
        crc = CRC16_X2[usize::from(x >> 8)] ^ CRC16_TABLE[usize::from(x & 0xFF)];
    }
    crc16_update_table(crc, tail)
}
// ANCHOR_END: crc_x2

// ANCHOR: decode_fast
/// Быстрый путь разбора: те же проверки, что у `telemetry_frame::decode`,
/// но без описания ошибки — `None` на любом отказе. Ошибку подробно
/// разберёт исходный код: отказ — редкость, и ускорять его незачем.
/// `X2` выбирает CRC: по два байта или по одному, как в проекте.
#[inline]
pub fn decode_fast<const X2: bool>(buf: &[u8]) -> Option<Frame<'_>> {
    let (head, rest) = buf.split_first_chunk::<HEADER_LEN>()?;
    let [magic, version, kind, seq, l0, l1, c0, c1] = *head;
    if magic != MAGIC || version != VERSION {
        return None;
    }
    let kind = Kind::try_from(kind).ok()?;
    let len = u16::from_le_bytes([l0, l1]);
    if usize::from(len) > MAX_PAYLOAD {
        return None;
    }
    let payload = rest.get(..usize::from(len))?;
    let crc =
        |crc, data| if X2 { crc16_update_x2(crc, data) } else { crc16_update_table(crc, data) };
    if u16::from_le_bytes([c0, c1]) != crc(crc(0xFFFF, &head[..6]), payload) {
        return None;
    }
    Some(Frame { header: Header { kind, seq, len }, payload })
}
// ANCHOR_END: decode_fast

// ANCHOR: hasher
/// Хеш для номера датчика: одно умножение на нечётную константу, как в
/// FxHash. Не стойкий: тот, кто выбирает номера датчиков, может выбрать
/// и коллизии. Номера приходят от своих устройств на шине — граница
/// доверия проходит раньше, у CRC и разбора кадра.
#[derive(Default)]
pub struct SensorHasher(u64);

impl Hasher for SensorHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u8(b);
        }
    }

    fn write_u8(&mut self, b: u8) {
        self.write_u64(u64::from(b));
    }

    fn write_u16(&mut self, n: u16) {
        self.write_u64(u64::from(n));
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0xF135_7AEA_2E62_A9C5);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
// ANCHOR_END: hasher

// ANCHOR: fast_store
/// Хранилище с тем же поведением, что `telemetry_store::Store`: кольцо
/// последних `cap` измерений на датчик. Отличия только в скорости:
/// разбор через [`decode_fast`] и хеш `S` вместо SipHash.
pub struct FastStore<S, const X2: bool> {
    sensors: HashMap<NonZeroU16, VecDeque<Point>, S>,
    cap: usize,
}

/// Свой цикл разбора, хеш прежний — SipHash из std.
pub type LoopStore = FastStore<std::hash::RandomState, false>;
/// Свой цикл и другой хеш.
pub type HashStore = FastStore<BuildHasherDefault<SensorHasher>, false>;
/// Свой цикл, другой хеш и CRC по два байта.
pub type X2Store = FastStore<BuildHasherDefault<SensorHasher>, true>;

impl<S: core::hash::BuildHasher + Default, const X2: bool> FastStore<S, X2> {
    pub fn new(cap: usize) -> Self {
        FastStore { sensors: HashMap::default(), cap: cap.max(1) }
    }

    pub fn ingest(&mut self, bytes: &[u8]) -> Ingested {
        let mut rep = Ingested::default();
        let mut pos = 0;
        while pos < bytes.len() {
            let frame = match decode_fast::<X2>(&bytes[pos..]) {
                Some(f) => {
                    pos += f.wire_len();
                    f
                }
                None => {
                    // Отказ: один шаг исходного итератора — та же ошибка и
                    // тот же поиск следующего маркера, что у Store::ingest.
                    let mut it = Frames::until_eof(&bytes[pos..]);
                    let item = it.next();
                    pos += it.position();
                    match item {
                        Some(Ok((_, f))) => f,
                        Some(Err(_)) => {
                            rep.errors += 1;
                            continue;
                        }
                        None => break,
                    }
                }
            };
            rep.frames[frame.header.kind as usize - 1] += 1;
            if frame.header.kind == Kind::Measurement {
                match Measurement::from_bytes(frame.payload) {
                    Ok(m) => {
                        let cap = self.cap;
                        let ring = self
                            .sensors
                            .entry(m.sensor)
                            .or_insert_with(|| VecDeque::with_capacity(cap));
                        if ring.len() == cap {
                            ring.pop_front();
                        }
                        ring.push_back(Point { t_ms: m.timestamp_ms, value: m.value });
                        rep.stored += 1;
                    }
                    Err(_) => rep.bad_payload += 1,
                }
            }
        }
        rep
    }

    /// Измерения датчика с временем в `[from_ms, to_ms]`.
    pub fn range(
        &self,
        sensor: NonZeroU16,
        from_ms: u32,
        to_ms: u32,
    ) -> impl Iterator<Item = &Point> {
        self.sensors.get(&sensor).into_iter().flat_map(move |r| {
            let lo = r.partition_point(|p| p.t_ms < from_ms);
            let hi = r.partition_point(|p| p.t_ms <= to_ms);
            r.range(lo..hi)
        })
    }

    /// Сводка, как у `Store::summary`.
    pub fn summary(&self) -> Summary {
        let mut s = Summary { sensors: self.sensors.len(), ..Summary::default() };
        for r in self.sensors.values() {
            s.points += r.len();
            if let Some(f) = r.front() {
                s.oldest_ms = Some(s.oldest_ms.map_or(f.t_ms, |o| o.min(f.t_ms)));
            }
            if let Some(b) = r.back() {
                s.newest_ms = Some(s.newest_ms.map_or(b.t_ms, |n| n.max(b.t_ms)));
            }
        }
        s
    }
}
// ANCHOR_END: fast_store

#[cfg(test)]
mod tests {
    use super::*;
    use telemetry_store::Store;

    fn xorshift(x: &mut u64) -> u64 {
        *x ^= *x << 13;
        *x ^= *x >> 7;
        *x ^= *x << 17;
        *x
    }

    /// Поток с порчей: перевёрнутые биты, вставленный мусор, обрыв.
    fn damaged(seed: u64) -> Vec<u8> {
        let mut x = seed;
        let mut s = stream(3000, 40);
        for _ in 0..30 {
            let i = (xorshift(&mut x) as usize) % s.len();
            s[i] ^= 1 << (xorshift(&mut x) % 8);
        }
        for _ in 0..5 {
            let i = (xorshift(&mut x) as usize) % s.len();
            let junk: Vec<u8> =
                (0..xorshift(&mut x) % 40).map(|_| xorshift(&mut x) as u8).collect();
            s.splice(i..i, junk);
        }
        s.truncate(s.len() - (xorshift(&mut x) as usize) % 17);
        s
    }

    fn same_as_store<S: core::hash::BuildHasher + Default, const X2: bool>(bytes: &[u8]) {
        let mut slow = Store::new(64);
        let mut fast = FastStore::<S, X2>::new(64);
        assert_eq!(slow.ingest(bytes), fast.ingest(bytes));
        assert_eq!(slow.summary(), fast.summary());
        for id in slow.sensors_sorted() {
            assert!(slow.range(id, 0, u32::MAX).eq(fast.range(id, 0, u32::MAX)));
            assert!(slow.range(id, 1000, 2000).eq(fast.range(id, 1000, 2000)));
        }
    }

    #[test]
    fn crc_x2_matches_bytewise() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 37 % 251) as u8).collect();
        for n in 0..data.len() {
            assert_eq!(crc16_update_x2(0xFFFF, &data[..n]), telemetry_frame::crc16(&data[..n]));
        }
        assert_eq!(crc16_update_x2(0xFFFF, b"123456789"), 0x29B1);
    }

    #[test]
    fn fast_stores_behave_like_store() {
        same_as_store::<std::hash::RandomState, false>(&stream(5000, 64));
        for seed in 1..=20 {
            let bytes = damaged(seed);
            same_as_store::<BuildHasherDefault<SensorHasher>, false>(&bytes);
            same_as_store::<BuildHasherDefault<SensorHasher>, true>(&bytes);
        }
    }

    #[test]
    fn stream_ingests_cleanly() {
        let s = stream(1600, 64);
        let mut store = Store::new(1024);
        let r = store.ingest(&s);
        assert_eq!(r.frames, [1500, 100, 0]);
        assert_eq!((r.stored, r.errors, r.bad_payload), (1500, 0, 0));
        assert_eq!(store.sensor_count(), 64);
    }
}
