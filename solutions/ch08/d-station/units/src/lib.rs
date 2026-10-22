//! Температура с датчика станции.
//!
//! Возможности `celsius` и `fahrenheit` добавляют по методу и ничего не
//! меняют в существующем коде. Шкалу выбирает тот, кто вызывает метод, а
//! не тот, кто включил возможность, поэтому унификация в рабочем
//! пространстве ничего не ломает.
use core::fmt;

/// Отсчёт термодатчика: сотые доли градуса Цельсия.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Temp(pub i32);

/// Значение в конкретной шкале: сотые доли градуса и обозначение.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    hundredths: i32,
    unit: &'static str,
}

// ANCHOR: additive
impl Temp {
    #[cfg(feature = "celsius")]
    pub fn celsius(self) -> Reading {
        Reading { hundredths: self.0, unit: "°C" }
    }

    #[cfg(feature = "fahrenheit")]
    pub fn fahrenheit(self) -> Reading {
        Reading { hundredths: self.0 * 9 / 5 + 3200, unit: "°F" }
    }
}
// ANCHOR_END: additive

impl Reading {
    /// Значение в сотых долях градуса своей шкалы.
    pub fn hundredths(self) -> i32 {
        self.hundredths
    }
}

impl fmt::Display for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.hundredths < 0 { "-" } else { "" };
        let v = self.hundredths.unsigned_abs();
        write!(f, "{sign}{}.{:02} {}", v / 100, v % 100, self.unit)
    }
}
