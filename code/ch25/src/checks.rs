//! Раздел 25.3: проверки при компиляции.
//!
//! `const _: () = assert!(…)` — аналог `_Static_assert` из C11, но условие
//! может вызывать любую `const fn`. Нарушение — ошибка E0080 с текстом из
//! `assert!`:
//!
//! ```compile_fail,E0080
//! pub const USED: usize = 13_000;
//! const _: () = assert!(USED <= 12_288, "буферы не помещаются");
//! ```
//!
//! Сообщение в константе — литерал или `"{}"` с единственным `&str`: число
//! не подставить, форматирование — вызов `Display::fmt`, метода трейта.
//!
//! ```compile_fail,E0015
//! pub const USED: usize = 13_000;
//! const _: () = if USED > 12_288 {
//!     panic!("занято {} байт", USED) // cannot call non-const formatting macro in constants
//! };
//! ```
//!
//! Проверка внутри обобщённой функции срабатывает при мономорфизации —
//! для того `N`, с которым функцию вызвали (раздел 10.4):
//!
//! ```compile_fail,E0080
//! let pool = ch25::checks::Pool::<40>::new(); // ёмкость Pool — не больше 32
//! ```
//!
//! ```
//! let mut pool = ch25::checks::Pool::<32>::new();
//! assert_eq!(pool.take(), Some(0));
//! ```

use core::mem::{align_of, offset_of, size_of};

// ANCHOR: layout
/// Заголовок записи журнала так, как его описывает C-заголовок
/// загрузчика:
///
/// ```c
/// struct log_header { uint16_t magic; uint8_t kind; uint8_t flags;
///                     uint32_t time; uint16_t len; uint16_t crc; };
/// ```
#[repr(C)]
pub struct LogHeader {
    pub magic: u16,
    pub kind: u8,
    pub flags: u8,
    pub time: u32,
    pub len: u16,
    pub crc: u16,
}

// Раскладка совпадает с C: размер, выравнивание, смещения. Если кто-то
// переставит поля или сменит тип, сборка остановится здесь, а не на
// разборе битого журнала.
const _: () = {
    assert!(size_of::<LogHeader>() == 12, "LogHeader: 12 байт, как struct log_header");
    assert!(align_of::<LogHeader>() == 4);
    assert!(offset_of!(LogHeader, time) == 4);
    assert!(offset_of!(LogHeader, len) == 8);
    assert!(offset_of!(LogHeader, crc) == 10);
};
// ANCHOR_END: layout

// ANCHOR: invariant
/// Пороги аварийных уровней температуры в десятых долях градуса. Поиск
/// уровня делением пополам верен, только если пороги строго возрастают.
pub static THRESHOLDS: [i16; 6] = [-400, -100, 0, 250, 600, 850];

const fn strictly_increasing(t: &[i16]) -> bool {
    let mut i = 1;
    while i < t.len() {
        if t[i - 1] >= t[i] {
            return false;
        }
        i += 1;
    }
    true
}

// Инвариант таблицы проверяется при сборке, а не при каждом запуске.
const _: () = assert!(strictly_increasing(&THRESHOLDS), "THRESHOLDS: пороги должны возрастать");

/// Номер уровня: сколько порогов не выше `t`.
pub fn level(t: i16) -> usize {
    THRESHOLDS.partition_point(|&x| x <= t)
}
// ANCHOR_END: invariant

// ANCHOR: generic
/// Пул из `N` ячеек; занятость — биты одного `u32`.
pub struct Pool<const N: usize> {
    used: u32,
    slots: [[u8; 16]; N],
}

impl<const N: usize> Pool<N> {
    pub const fn new() -> Self {
        // Проверка для каждого N, с которым создают пул.
        const {
            assert!(N <= 32, "ёмкость Pool — не больше 32: занятость хранится в u32")
        };
        Pool { used: 0, slots: [[0; 16]; N] }
    }

    /// Номер свободной ячейки.
    pub fn take(&mut self) -> Option<usize> {
        let free = (!self.used).trailing_zeros() as usize;
        if free >= N {
            return None;
        }
        self.used |= 1 << free;
        Some(free)
    }

    pub fn slot(&mut self, i: usize) -> &mut [u8; 16] {
        &mut self.slots[i]
    }
}

impl<const N: usize> Default for Pool<N> {
    fn default() -> Self {
        Self::new()
    }
}
// ANCHOR_END: generic

// ANCHOR: cfg
// Аналог `#error`: сборка для цели, на которой код не имеет смысла,
// останавливается сразу и с понятным текстом.
#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("ch25: нужны 32- или 64-битные адреса");
// ANCHOR_END: cfg

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(level(-500), 0);
        assert_eq!(level(-400), 1);
        assert_eq!(level(249), 3);
        assert_eq!(level(900), 6);
        assert!(strictly_increasing(&THRESHOLDS));
        assert!(!strictly_increasing(&[1, 1]));
    }

    #[test]
    fn pool() {
        let mut p = Pool::<3>::new();
        assert_eq!([p.take(), p.take(), p.take(), p.take()], [Some(0), Some(1), Some(2), None]);
        p.slot(1)[0] = 7;
        assert_eq!(p.slot(1)[0], 7);
        let mut full = Pool::<32>::default();
        assert_eq!((0..33).filter_map(|_| full.take()).count(), 32);
    }
}
