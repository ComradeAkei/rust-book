//! Раздел 11.7: закрытый набор типов — перечисление и `match` вместо
//! `dyn`.

use crate::fat::{Counter, Sensor, Thermo};

// ANCHOR: enum
/// Все датчики станции известны заранее: вариант на каждый тип.
#[derive(Clone, Copy)]
pub enum AnySensor {
    Thermo(Thermo),
    Counter(Counter),
}

impl Sensor for AnySensor {
    fn read(&self) -> i32 {
        match self {
            AnySensor::Thermo(t) => t.read(),
            AnySensor::Counter(c) => c.read(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            AnySensor::Thermo(t) => t.name(),
            AnySensor::Counter(c) => c.name(),
        }
    }
}

impl From<Thermo> for AnySensor {
    fn from(t: Thermo) -> Self {
        AnySensor::Thermo(t)
    }
}

impl From<Counter> for AnySensor {
    fn from(c: Counter) -> Self {
        AnySensor::Counter(c)
    }
}
// ANCHOR_END: enum

// ANCHOR: read
/// Вызов через перечисление: сравнение тега вместо загрузки из таблицы.
#[unsafe(no_mangle)]
pub fn ch11_read_enum(s: &AnySensor) -> i32 {
    s.read()
}
// ANCHOR_END: read

// ANCHOR: poll
/// Тот же опрос, что `ch11_poll_dyn`, но по значениям в массиве: ни кучи, ни
/// таблиц, оба тела `read` встроены.
#[unsafe(no_mangle)]
pub fn ch11_poll_enum(sensors: &[AnySensor]) -> i32 {
    let mut sum = 0i32;
    for s in sensors {
        sum = sum.wrapping_add(s.read());
    }
    sum
}
// ANCHOR_END: poll

// ANCHOR: station
/// Станция без кучи: шесть датчиков в одном массиве на стеке или во flash.
pub const STATION: [AnySensor; 6] = [
    AnySensor::Thermo(Thermo { raw: 344 }),
    AnySensor::Counter(Counter { pulses: 7 }),
    AnySensor::Thermo(Thermo { raw: -40 }),
    AnySensor::Counter(Counter { pulses: 0 }),
    AnySensor::Thermo(Thermo { raw: 16 }),
    AnySensor::Counter(Counter { pulses: 1 }),
];
// ANCHOR_END: station

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_dispatch() {
        assert_eq!(ch11_poll_enum(&STATION), 2150 + 7 - 250 + 100 + 1);
        let a: AnySensor = Thermo { raw: 32 }.into();
        assert_eq!((a.read(), a.name()), (200, "термодатчик"));
        assert_eq!(ch11_read_enum(&a), 200);
        let b = AnySensor::from(Counter { pulses: 2 });
        assert_eq!((b.read(), b.name()), (2, "датчик"));
        // Перечисление не больше самого большого варианта плюс тег.
        assert!(size_of::<AnySensor>() <= 8);
    }
}
