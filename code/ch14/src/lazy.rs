//! Раздел 14.6: ленивая инициализация.
//!
//! `OnceCell` для значения, которое считается при первом обращении;
//! `LazyCell` для того же с функцией, известной заранее; в `std` — их
//! потокобезопасные варианты `OnceLock` и `LazyLock` для `static`.

use core::cell::{Cell, LazyCell, OnceCell};

// ANCHOR: channel
/// Канал АЦП с калибровкой по двум точкам. Коэффициенты считаются при
/// первом пересчёте и запоминаются; `&self` хватает, потому что
/// `OnceCell` можно заполнить через общую ссылку — один раз.
pub struct Channel {
    /// Две точки калибровки: отсчёт АЦП и величина в сотых долях.
    points: [(u16, i32); 2],
    /// Наклон в 1/65536 и смещение.
    coeffs: OnceCell<(i64, i64)>,
    pub fits: Cell<u32>,
}

impl Channel {
    pub const fn new(points: [(u16, i32); 2]) -> Self {
        Self { points, coeffs: OnceCell::new(), fits: Cell::new(0) }
    }

    fn fit(&self) -> (i64, i64) {
        self.fits.set(self.fits.get() + 1);
        let [(x0, y0), (x1, y1)] = self.points.map(|(x, y)| (i64::from(x), i64::from(y)));
        let slope = ((y1 - y0) << 16) / (x1 - x0);
        (slope, y0 - ((slope * x0) >> 16))
    }

    pub fn value(&self, raw: u16) -> i32 {
        let &(slope, offset) = self.coeffs.get_or_init(|| self.fit());
        ((slope * i64::from(raw)) >> 16) as i32 + offset as i32
    }
}
// ANCHOR_END: channel

// ANCHOR: lazycell
/// Таблица, которая нужна не всегда: `LazyCell` построит её при первом
/// разыменовании и ни разу, если до таблицы дело не дошло.
pub fn checksum_if(data: &[u8], needed: bool, builds: &Cell<u32>) -> Option<u16> {
    let table = LazyCell::new(|| {
        builds.set(builds.get() + 1);
        crc_table()
    });
    needed.then(|| crc16_with(&table, data))
}
// ANCHOR_END: lazycell

// ANCHOR: crc
/// Таблица CRC-16/CCITT (полином 0x1021) на 256 входов — 512 байт.
pub fn crc_table() -> [u16; 256] {
    core::array::from_fn(|i| {
        let mut c = (i as u16) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
        c
    })
}

/// Байт за шаг вместо бита за шаг, начальное значение 0xFFFF.
pub fn crc16_with(table: &[u16; 256], data: &[u8]) -> u16 {
    data.iter().fold(0xFFFF, |crc, &b| (crc << 8) ^ table[usize::from((crc >> 8) as u8 ^ b)])
}
// ANCHOR_END: crc

#[cfg(feature = "std")]
// ANCHOR: lazylock
/// Общая для всех потоков таблица: строится при первом обращении любого
/// из них, остальные ждут, пока она будет готова.
pub static CRC_TABLE: std::sync::LazyLock<[u16; 256]> = std::sync::LazyLock::new(crc_table);

#[cfg(feature = "std")]
pub fn crc16(data: &[u8]) -> u16 {
    crc16_with(&CRC_TABLE, data)
}
// ANCHOR_END: lazylock

#[cfg(feature = "std")]
// ANCHOR: oncelock
/// Имя станции задаётся один раз при старте и читается отовсюду.
static STATION: std::sync::OnceLock<std::string::String> = std::sync::OnceLock::new();

/// Второй вызов не меняет имя и возвращает отвергнутое.
#[cfg(feature = "std")]
pub fn set_station(name: &str) -> Result<(), std::string::String> {
    STATION.set(name.into())
}

#[cfg(feature = "std")]
pub fn station() -> Option<&'static str> {
    STATION.get().map(|s| s.as_str())
}
// ANCHOR_END: oncelock

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_fits_once() {
        // 0 отсчётов — −40,00 °C, 4095 — 125,00 °C.
        let ch = Channel::new([(0, -4000), (4095, 12500)]);
        assert_eq!(ch.fits.get(), 0);
        assert_eq!(ch.value(0), -4000);
        assert_eq!(ch.value(4095), 12499); // наклон округлён вниз
        assert_eq!(ch.value(2048), 4252);
        assert_eq!(ch.fits.get(), 1);
    }

    #[test]
    fn lazycell_builds_only_when_needed() {
        let builds = Cell::new(0);
        assert_eq!(checksum_if(b"123456789", false, &builds), None);
        assert_eq!(builds.get(), 0);
        assert_eq!(checksum_if(b"123456789", true, &builds), Some(0x29B1));
        assert_eq!(builds.get(), 1);
    }

    #[test]
    #[cfg(feature = "std")]
    fn table_crc_matches_project() {
        for data in [&b""[..], b"123456789", &[0xA5; 300]] {
            assert_eq!(crc16(data), telemetry_frame::crc16(data));
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn station_set_once() {
        assert_eq!(set_station("Обнинск-1"), Ok(()));
        assert_eq!(set_station("другая"), Err("другая".into()));
        assert_eq!(station(), Some("Обнинск-1"));
    }

    #[test]
    #[should_panic(expected = "reentrant init")]
    fn oncecell_reentrant_init_panics() {
        let c: OnceCell<u32> = OnceCell::new();
        c.get_or_init(|| *c.get_or_init(|| 1) + 1);
    }
}
