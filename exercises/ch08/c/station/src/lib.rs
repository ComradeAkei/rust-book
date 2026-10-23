//! Задача 8.C: библиотека станции для программы на C.
//!
//! Внутренние функции — обычный Rust, они уже готовы. Допишите обёртки,
//! которые объявлены в `../station.h`. Внутренние функции помечены
//! `#[inline(never)]`, чтобы их символы остались в архиве и их можно было
//! рассмотреть через nm.

/// Множитель датчика: отсчёт × num / den.
pub struct Scale {
    num: i32,
    den: i32,
}

impl Scale {
    #[inline(never)]
    pub fn apply(&self, raw: i32) -> i32 {
        raw.saturating_mul(self.num) / self.den
    }
}

/// Датчики станции, как в `code/ch08/sensors.csv`.
static SCALES: [(u16, Scale); 4] = [
    (1, Scale { num: 10, den: 1 }),
    (2, Scale { num: 10, den: 1 }),
    (7, Scale { num: 25, den: 1 }),
    (12, Scale { num: 2, den: 1 }),
];

#[inline(never)]
pub fn scale_for(id: u16) -> Option<&'static Scale> {
    SCALES.iter().find(|(i, _)| *i == id).map(|(_, s)| s)
}

/// CRC-8 с полиномом 0x07 без таблицы.
#[inline(never)]
pub fn crc8<T: AsRef<[u8]>>(data: T) -> u8 {
    data.as_ref().iter().fold(0, |crc, &b| {
        let mut c = crc ^ b;
        for _ in 0..8 {
            c = if c & 0x80 != 0 { (c << 1) ^ 0x07 } else { c << 1 };
        }
        c
    })
}

// TODO(задача C): station_crc8 и station_scale из station.h.
