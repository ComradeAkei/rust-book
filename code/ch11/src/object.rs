//! Раздел 11.4: объекты в куче и без неё. `Box<dyn>`, `Rc<dyn>`,
//! пересылающие реализации и обработчик `&mut dyn FnMut` без аллокации.
//!
//! Обобщённая функция без `?Sized` объект не примет: неявное ограничение
//! `S: Sized` не выполняется для `dyn Sensor`.
//!
//! ```compile_fail,E0277
//! use ch11::fat::{Sensor, Thermo};
//! fn twice<S: Sensor>(s: &S) -> i32 {
//!     s.read() * 2
//! }
//! let d: &dyn Sensor = &Thermo { raw: 16 };
//! twice(d);
//! ```

use crate::fat::Sensor;

// ANCHOR: unsized
/// `?Sized` снимает неявное ограничение: функция принимает и `&Thermo`, и
/// `&dyn Sensor`. Копия для `dyn Sensor` вызывает `read` через таблицу.
pub fn report<S: Sensor + ?Sized>(s: &S) -> (&'static str, i32) {
    (s.name(), s.read())
}
// ANCHOR_END: unsized

// ANCHOR: forward
/// Пересылающая реализация: ссылка на датчик — тоже датчик. С ней
/// обобщённый код, который берёт `S: Sensor` по значению, примет и
/// `&Thermo`, и `&dyn Sensor`.
impl<S: Sensor + ?Sized> Sensor for &S {
    fn read(&self) -> i32 {
        (**self).read()
    }

    fn name(&self) -> &'static str {
        (**self).name()
    }
}

pub fn sum_all<S: Sensor>(sensors: impl IntoIterator<Item = S>) -> i32 {
    sensors.into_iter().map(|s| s.read()).sum()
}
// ANCHOR_END: forward

// ANCHOR: callback
/// Обход отсчётов с обработчиком тревоги. Обработчик — объект: функция
/// одна на все замыкания, а само замыкание лежит на стеке вызывающего.
pub fn scan(samples: &[i16], limit: i16, on_alarm: &mut dyn FnMut(usize, i16)) -> usize {
    let mut alarms = 0;
    for (i, &s) in samples.iter().enumerate() {
        if s > limit {
            alarms += 1;
            on_alarm(i, s);
        }
    }
    alarms
}

#[unsafe(no_mangle)]
pub fn ch11_scan_max(samples: &[i16], limit: i16) -> i16 {
    let mut worst = i16::MIN;
    scan(samples, limit, &mut |_, s| worst = worst.max(s));
    worst
}
// ANCHOR_END: callback

#[cfg(feature = "alloc")]
pub use heap::*;

#[cfg(feature = "alloc")]
mod heap {
    use crate::fat::{Counter, Sensor, Thermo};
    use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};

    // ANCHOR: box
    /// Пересылка для `Box`: `Box<dyn Sensor>` тоже датчик.
    impl<S: Sensor + ?Sized> Sensor for Box<S> {
        fn read(&self) -> i32 {
            (**self).read()
        }

        fn name(&self) -> &'static str {
            (**self).name()
        }
    }

    /// Станция с датчиками разных типов: в векторе лежат толстые
    /// указатели, сами датчики — в куче, каждый своего размера.
    pub fn station() -> Vec<Box<dyn Sensor>> {
        vec![
            Box::new(Thermo { raw: 344 }),
            Box::new(Counter { pulses: 7 }),
            Box::new(Thermo { raw: -40 }),
        ]
    }
    // ANCHOR_END: box

    // ANCHOR: rc
    /// `Rc<dyn Sensor>` указывает на блок со счётчиками и значением.
    /// Где в блоке начинается значение, зависит от его выравнивания, и
    /// узнать это можно только из таблицы.
    #[unsafe(no_mangle)]
    pub fn ch11_rc_read(r: &Rc<dyn Sensor>) -> i32 {
        r.read()
    }

    pub fn shared() -> [Rc<dyn Sensor>; 2] {
        let t: Rc<dyn Sensor> = Rc::new(Thermo { raw: 16 });
        [t.clone(), t]
    }
    // ANCHOR_END: rc

    // ANCHOR: double
    /// Размер значения за объектом. `&Box<dyn Sensor>` можно привести к
    /// `&dyn Sensor` двумя путями: разыменовать `Box` или сделать объектом
    /// сам `Box`, раз он тоже датчик. Компилятор выбирает второй.
    pub fn object_size(s: &dyn Sensor) -> usize {
        core::mem::size_of_val(s)
    }

    pub fn boxed_sizes() -> (usize, usize) {
        let b: Box<dyn Sensor> = Box::new(Thermo { raw: 5 });
        (object_size(&b), object_size(&*b))
    }
    // ANCHOR_END: double
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fat::{Counter, Thermo};
    use std::rc::Rc;

    #[test]
    fn unsized_and_forwarding() {
        let t = Thermo { raw: 16 };
        let d: &dyn Sensor = &t;
        assert_eq!(report(&t), ("термодатчик", 100));
        assert_eq!(report(d), ("термодатчик", 100));
        let c = Counter { pulses: 5 };
        let list: [&dyn Sensor; 2] = [&t, &c];
        assert_eq!(sum_all(list), 105);
        assert_eq!(sum_all([&t, &t]), 200);
        assert_eq!(sum_all(station()), 2150 + 7 - 250);
        assert_eq!((&&t).name(), "термодатчик");
    }

    #[test]
    fn callbacks() {
        let s = [10, 300, -5, 900, 120];
        assert_eq!(ch11_scan_max(&s, 100), 900);
        assert_eq!(ch11_scan_max(&s, 1000), i16::MIN);
        let mut seen = Vec::new();
        assert_eq!(scan(&s, 100, &mut |i, v| seen.push((i, v))), 3);
        assert_eq!(seen, [(1, 300), (3, 900), (4, 120)]);
    }

    #[test]
    fn rc_objects() {
        let [a, b] = shared();
        assert_eq!(Rc::strong_count(&a), 2);
        assert_eq!(ch11_rc_read(&a) + ch11_rc_read(&b), 200);
        assert!(Rc::ptr_eq(&a, &b));
    }

    #[test]
    fn box_becomes_object() {
        let words = core::mem::size_of::<usize>();
        assert_eq!(boxed_sizes(), (2 * words, 2));
    }
}
