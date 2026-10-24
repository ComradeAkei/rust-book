//! Раздел 31.4: команды хоста для станции по UART. Ошибка в разборе
//! калибровки лежит за маркером, кодом команды, длиной и CRC: случайные
//! байты до неё не доходят, а фаззер с обратной связью по покрытию — да.
//!
//! ```text
//! смещение  размер  поле
//! 0         1       sync = 0x5A
//! 1         1       cmd  (1 ping, 2 период, 3 калибровка, 4 сброс)
//! 2         1       len  длина нагрузки
//! 3         len     payload
//! 3 + len   1       crc  CRC-8/SMBUS по cmd, len и нагрузке
//! ```

use core::fmt;
use core::num::NonZeroU16;

pub const SYNC: u8 = 0x5A;
pub const MAX_POINTS: usize = 8;

// ANCHOR: types
/// Точка калибровки: сырое значение АЦП и физическое.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Point {
    pub raw: i16,
    pub value: i16,
}

/// Таблица калибровки на `n` точек; место — под `MAX_POINTS`, как в
/// прошивке без кучи.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Table {
    n: u8,
    points: [Point; MAX_POINTS],
}

impl Table {
    pub fn new(points: &[Point]) -> Option<Table> {
        let mut t = Table { n: 0, points: [Point::default(); MAX_POINTS] };
        t.points.get_mut(..points.len())?.copy_from_slice(points);
        t.n = points.len() as u8;
        Some(t)
    }

    pub fn points(&self) -> &[Point] {
        &self.points[..usize::from(self.n)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    Ping,
    SetPeriod { sensor: NonZeroU16, period_ms: u16 },
    Calibrate { sensor: NonZeroU16, table: Table },
    Reset,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmdError {
    Short,
    BadSync(u8),
    BadCrc { stored: u8, computed: u8 },
    Unknown(u8),
    BadLength { cmd: u8, len: usize },
    BadSensor,
    TooManyPoints(usize),
}
// ANCHOR_END: types

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            CmdError::Short => f.write_str("команда не полна"),
            CmdError::BadSync(b) => write!(f, "неверный маркер {b:#04x}"),
            CmdError::BadCrc { stored, computed } => {
                write!(f, "CRC не совпадает: в команде {stored:#04x}, посчитано {computed:#04x}")
            }
            CmdError::Unknown(c) => write!(f, "неизвестная команда {c}"),
            CmdError::BadLength { cmd, len } => write!(f, "команда {cmd}: нагрузка {len} байт"),
            CmdError::BadSensor => f.write_str("датчик 0 не существует"),
            CmdError::TooManyPoints(n) => write!(f, "точек калибровки {n}, больше {MAX_POINTS}"),
        }
    }
}

impl std::error::Error for CmdError {}

// ANCHOR: crc8
/// CRC-8/SMBUS: полином 0x07, начальное значение 0. Контрольное значение
/// для "123456789" — 0xF4.
pub const fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    let mut i = 0;
    while i < data.len() {
        crc ^= data[i];
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x80 != 0 { (crc << 1) ^ 0x07 } else { crc << 1 };
            bit += 1;
        }
        i += 1;
    }
    crc
}

const _: () = assert!(crc8(b"123456789") == 0xF4);
// ANCHOR_END: crc8

// ANCHOR: frame
/// Проверить рамку команды и отдать код с нагрузкой.
fn unframe(data: &[u8]) -> Result<(u8, &[u8]), CmdError> {
    let [sync, cmd, len, rest @ ..] = data else {
        return Err(CmdError::Short);
    };
    if *sync != SYNC {
        return Err(CmdError::BadSync(*sync));
    }
    let len = usize::from(*len);
    let (payload, crc) = rest.split_at_checked(len).ok_or(CmdError::Short)?;
    let &[stored, ..] = crc else {
        return Err(CmdError::Short);
    };
    let computed = crc8(&data[1..3 + len]);
    if stored != computed {
        return Err(CmdError::BadCrc { stored, computed });
    }
    Ok((*cmd, payload))
}

fn sensor(lo: u8, hi: u8) -> Result<NonZeroU16, CmdError> {
    NonZeroU16::new(u16::from_le_bytes([lo, hi])).ok_or(CmdError::BadSensor)
}

fn parse_with(
    data: &[u8],
    calibrate: fn(NonZeroU16, u8, &[u8]) -> Result<Table, CmdError>,
) -> Result<Command, CmdError> {
    let (cmd, payload) = unframe(data)?;
    let bad_len = CmdError::BadLength { cmd, len: payload.len() };
    match (cmd, payload) {
        (1, []) => Ok(Command::Ping),
        (2, &[s0, s1, p0, p1]) => Ok(Command::SetPeriod {
            sensor: sensor(s0, s1)?,
            period_ms: u16::from_le_bytes([p0, p1]),
        }),
        (3, &[s0, s1, n, ref points @ ..]) => {
            let sensor = sensor(s0, s1)?;
            let table = calibrate(sensor, n, points)?;
            Ok(Command::Calibrate { sensor, table })
        }
        (4, []) => Ok(Command::Reset),
        (1..=4, _) => Err(bad_len),
        (other, _) => Err(CmdError::Unknown(other)),
    }
}
// ANCHOR_END: frame

// ANCHOR: v1
/// Точки калибровки: `n` раз по четыре байта.
fn calibrate_v1(_: NonZeroU16, n: u8, bytes: &[u8]) -> Result<Table, CmdError> {
    let n = usize::from(n);
    if bytes.len() != 4 * n {
        return Err(CmdError::BadLength { cmd: 3, len: 3 + bytes.len() });
    }
    let mut table = Table { n: n as u8, points: [Point::default(); MAX_POINTS] };
    for (i, p) in bytes.chunks_exact(4).enumerate() {
        table.points[i] = Point {
            raw: i16::from_le_bytes([p[0], p[1]]),
            value: i16::from_le_bytes([p[2], p[3]]),
        };
    }
    Ok(table)
}

pub fn parse_v1(data: &[u8]) -> Result<Command, CmdError> {
    parse_with(data, calibrate_v1)
}
// ANCHOR_END: v1

// ANCHOR: fixed
fn calibrate(sensor: NonZeroU16, n: u8, bytes: &[u8]) -> Result<Table, CmdError> {
    if usize::from(n) > MAX_POINTS {
        return Err(CmdError::TooManyPoints(n.into()));
    }
    calibrate_v1(sensor, n, bytes)
}

pub fn parse(data: &[u8]) -> Result<Command, CmdError> {
    parse_with(data, calibrate)
}
// ANCHOR_END: fixed

// ANCHOR: encode
impl Command {
    /// Команда на проводе; самая длинная — 3 + 3 + 4·8 + 1 = 39 байт.
    pub fn encode(&self, out: &mut [u8; 39]) -> usize {
        let mut payload = [0u8; 35];
        let (cmd, len) = match *self {
            Command::Ping => (1, 0),
            Command::SetPeriod { sensor, period_ms } => {
                payload[..2].copy_from_slice(&sensor.get().to_le_bytes());
                payload[2..4].copy_from_slice(&period_ms.to_le_bytes());
                (2, 4)
            }
            Command::Calibrate { sensor, ref table } => {
                payload[..2].copy_from_slice(&sensor.get().to_le_bytes());
                payload[2] = table.n;
                for (i, p) in table.points().iter().enumerate() {
                    payload[3 + 4 * i..][..2].copy_from_slice(&p.raw.to_le_bytes());
                    payload[5 + 4 * i..][..2].copy_from_slice(&p.value.to_le_bytes());
                }
                (3, 3 + 4 * table.points().len())
            }
            Command::Reset => (4, 0),
        };
        out[..3].copy_from_slice(&[SYNC, cmd, len as u8]);
        out[3..3 + len].copy_from_slice(&payload[..len]);
        out[3 + len] = crc8(&out[1..3 + len]);
        4 + len
    }
}
// ANCHOR_END: encode

#[cfg(feature = "arbitrary")]
// ANCHOR: arbitrary
impl<'a> arbitrary::Arbitrary<'a> for Command {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let sensor = NonZeroU16::new(u.int_in_range(1..=u16::MAX)?).unwrap();
        Ok(match u.int_in_range(1..=4)? {
            1 => Command::Ping,
            2 => Command::SetPeriod { sensor, period_ms: u.arbitrary()? },
            3 => {
                let n = u.int_in_range(0..=MAX_POINTS)?;
                let mut points = [Point::default(); MAX_POINTS];
                for p in &mut points[..n] {
                    *p = Point { raw: u.arbitrary()?, value: u.arbitrary()? };
                }
                Command::Calibrate { sensor, table: Table::new(&points[..n]).unwrap() }
            }
            _ => Command::Reset,
        })
    }
}
// ANCHOR_END: arbitrary

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(cmd: &Command) -> Vec<u8> {
        let mut out = [0; 39];
        let n = cmd.encode(&mut out);
        out[..n].to_vec()
    }

    #[test]
    fn known_commands() {
        assert_eq!(wire(&Command::Ping), [SYNC, 1, 0, crc8(&[1, 0])]);
        let s7 = NonZeroU16::new(7).unwrap();
        let period = Command::SetPeriod { sensor: s7, period_ms: 500 };
        assert_eq!(wire(&period)[..7], [SYNC, 2, 4, 7, 0, 0xF4, 0x01]);
        let pts = [Point { raw: -100, value: 0 }, Point { raw: 3000, value: 250 }];
        let cal = Command::Calibrate { sensor: s7, table: Table::new(&pts).unwrap() };
        for c in [Command::Ping, period, cal, Command::Reset] {
            assert_eq!(parse(&wire(&c)), Ok(c));
            assert_eq!(parse_v1(&wire(&c)), Ok(c));
        }
        // Самая длинная команда — ровно 39 байт.
        let full = Table::new(&[Point::default(); MAX_POINTS]).unwrap();
        assert_eq!(wire(&Command::Calibrate { sensor: s7, table: full }).len(), 39);
        assert_eq!(Table::new(&[Point::default(); MAX_POINTS + 1]), None);
    }

    #[test]
    fn errors() {
        assert_eq!(parse(&[]), Err(CmdError::Short));
        assert_eq!(parse(&[0x00, 1, 0, 0]), Err(CmdError::BadSync(0)));
        assert_eq!(parse(&[SYNC, 1, 0]), Err(CmdError::Short));
        assert_eq!(parse(&[SYNC, 1, 2, 0]), Err(CmdError::Short));
        let crc = crc8(&[1, 0]);
        assert_eq!(
            parse(&[SYNC, 1, 0, !crc]),
            Err(CmdError::BadCrc { stored: !crc, computed: crc })
        );
        assert_eq!(parse(&[SYNC, 9, 0, crc8(&[9, 0])]), Err(CmdError::Unknown(9)));
        assert_eq!(
            parse(&[SYNC, 1, 1, 0, crc8(&[1, 1, 0])]),
            Err(CmdError::BadLength { cmd: 1, len: 1 })
        );
        let zero = [SYNC, 2, 4, 0, 0, 1, 0, crc8(&[2, 4, 0, 0, 1, 0])];
        assert_eq!(parse(&zero), Err(CmdError::BadSensor));
    }

    /// Девять точек: исправленный разбор отвечает ошибкой, первая версия
    /// паникует на записи девятой точки в таблицу на восемь.
    #[test]
    fn nine_points() {
        let mut data = vec![SYNC, 3, 3 + 36, 7, 0, 9];
        data.extend([0u8; 36]);
        data.push(crc8(&data[1..]));
        assert_eq!(parse(&data), Err(CmdError::TooManyPoints(9)));
        let caught = std::panic::catch_unwind(|| parse_v1(&data));
        assert!(caught.is_err());
    }
}
