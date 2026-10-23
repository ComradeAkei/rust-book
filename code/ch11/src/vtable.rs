//! Раздел 11.2: что лежит в таблице методов и как через неё работают
//! вызов, `drop` и освобождение памяти.

use crate::fat::{Counter, Sensor, Thermo};

// ANCHOR: statics
/// Таблица датчиков, известная при компиляции. И объекты, и их таблицы
/// методов — константы: на микроконтроллере они остаются во flash.
pub static THERMO: Thermo = Thermo { raw: 344 };
pub static PULSES: Counter = Counter { pulses: 12 };
pub static SENSORS: [&(dyn Sensor + Sync); 2] = [&THERMO, &PULSES];
// ANCHOR_END: statics

// ANCHOR: as-dyn
/// Приведение `&Thermo` к `&dyn Sensor`: к адресу объекта добавляется
/// адрес таблицы, которую компилятор построил для пары `Thermo`, `Sensor`.
#[unsafe(no_mangle)]
pub fn ch11_as_dyn(t: &Thermo) -> &dyn Sensor {
    t
}
// ANCHOR_END: as-dyn

// ANCHOR: name
/// Метод по умолчанию тоже занимает место в таблице: у `Counter` там
/// лежит `name` из трейта, у `Thermo` — своя.
#[unsafe(no_mangle)]
pub fn ch11_name_dyn(s: &dyn Sensor) -> &'static str {
    s.name()
}
// ANCHOR_END: name

// ANCHOR: twice
/// Два вызова подряд: адрес `read` читается из таблицы один раз.
#[unsafe(no_mangle)]
pub fn ch11_read_twice(s: &dyn Sensor) -> i32 {
    s.read().wrapping_add(s.read())
}
// ANCHOR_END: twice

#[cfg(feature = "alloc")]
pub use boxed::*;

#[cfg(feature = "alloc")]
mod boxed {
    use crate::fat::Sensor;
    use alloc::boxed::Box;
    use core::cell::Cell;

    // ANCHOR: drop
    /// Датчик с деструктором: в слоте `drop` его таблицы — `drop_in_place`.
    pub struct Logged<'a> {
        pub drops: &'a Cell<u32>,
        pub last: i32,
    }

    impl Sensor for Logged<'_> {
        fn read(&self) -> i32 {
            self.last
        }
    }

    impl Drop for Logged<'_> {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    /// Освобождение `Box<dyn Sensor>`: деструктор и размер с выравниванием
    /// для `dealloc` берутся из таблицы.
    #[unsafe(no_mangle)]
    pub fn ch11_drop_box(b: Box<dyn Sensor + '_>) {
        drop(b);
    }
    // ANCHOR_END: drop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statics_and_defaults() {
        let sum: i32 = SENSORS.iter().map(|s| s.read()).sum();
        assert_eq!(sum, 2162);
        assert_eq!(ch11_name_dyn(SENSORS[0]), "термодатчик");
        assert_eq!(ch11_name_dyn(SENSORS[1]), "датчик");
        assert_eq!(ch11_as_dyn(&THERMO).read(), 2150);
        assert_eq!(ch11_read_twice(&PULSES), 24);
    }

    #[test]
    fn drop_through_vtable() {
        let drops = core::cell::Cell::new(0);
        let b: Box<dyn Sensor + '_> = Box::new(Logged { drops: &drops, last: 5 });
        assert_eq!(b.read(), 5);
        ch11_drop_box(b);
        ch11_drop_box(Box::new(Thermo { raw: 0 }));
        assert_eq!(drops.get(), 1);
    }
}
