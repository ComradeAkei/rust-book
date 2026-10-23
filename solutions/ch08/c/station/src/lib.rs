//! Задача 8.C: библиотека станции для программы на C.
//!
//! Внутренние функции — обычный Rust. Для C экспортированы обёртки из
//! `station.h`: соглашение о вызовах C и имена без манглинга. Внутренние
//! функции помечены `#[inline(never)]`, чтобы их символы остались в
//! архиве и их можно было рассмотреть через nm.

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

// ANCHOR: export
/// # Safety
///
/// `data` указывает на `len` читаемых байт; при `len == 0` может быть
/// нулевым.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn station_crc8(data: *const u8, len: usize) -> u8 {
    if len == 0 {
        // from_raw_parts не принимает нулевой указатель даже для пустого среза.
        return crc8([0u8; 0]);
    }
    // SAFETY: по контракту выше data указывает на len байт.
    crc8(unsafe { core::slice::from_raw_parts(data, len) })
}

/// # Safety
///
/// `out` указывает на `int32_t`, доступный для записи.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn station_scale(id: u16, raw: i32, out: *mut i32) -> i32 {
    match scale_for(id) {
        Some(s) => {
            // SAFETY: по контракту выше out можно писать.
            unsafe { out.write(s.apply(raw)) };
            0
        }
        None => -1,
    }
}
// ANCHOR_END: export
