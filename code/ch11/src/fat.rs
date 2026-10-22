//! Раздел 11.1: `dyn Trait`, приведение к объекту трейта и толстые
//! указатели.
//!
//! У `dyn Sensor` нет размера, известного при компиляции: значение такого
//! типа нельзя положить в переменную, только за указатель.
//!
//! ```compile_fail,E0277
//! use ch11::fat::{Sensor, Thermo};
//! let s: dyn Sensor = Thermo { raw: 344 };
//! ```
//!
//! `Box<dyn Sensor>` — это `Box<dyn Sensor + 'static>`: объект, который
//! держит заём, туда не положить.
//!
//! ```compile_fail
//! use ch11::fat::{Sensor, Window};
//! fn boxed(buf: &[i16]) -> Box<dyn Sensor> {
//!     Box::new(Window { buf })
//! }
//! ```
//!
//! Явное время жизни объекта снимает запрет:
//!
//! ```
//! use ch11::fat::{Sensor, Window};
//! fn boxed<'a>(buf: &'a [i16]) -> Box<dyn Sensor + 'a> {
//!     Box::new(Window { buf })
//! }
//! assert_eq!(boxed(&[10, 20, 30]).read(), 20);
//! ```
//!
//! `dyn Sensor` ничего не обещает про потоки. Чтобы передать объект в
//! другой поток, автотрейт дописывают к типу объекта:
//!
//! ```compile_fail,E0277
//! use ch11::fat::Sensor;
//! fn spawn_poll(s: Box<dyn Sensor>) {
//!     std::thread::spawn(move || s.read());
//! }
//! ```
//!
//! ```
//! use ch11::fat::{Sensor, Thermo};
//! fn spawn_poll(s: Box<dyn Sensor + Send>) -> i32 {
//!     std::thread::spawn(move || s.read()).join().unwrap()
//! }
//! assert_eq!(spawn_poll(Box::new(Thermo { raw: 344 })), 2150);
//! ```

// ANCHOR: sensor
/// Источник отсчётов. В этой главе `read` берёт `&self`: объекты будут
/// лежать и во flash, где менять нечего.
pub trait Sensor {
    /// Очередной отсчёт в единицах датчика.
    fn read(&self) -> i32;

    /// Имя для журнала.
    fn name(&self) -> &'static str {
        "датчик"
    }
}

/// Термодатчик вроде DS18B20: отсчёт в шестнадцатых долях градуса,
/// `read` отдаёт сотые.
#[derive(Clone, Copy)]
pub struct Thermo {
    pub raw: i16,
}

impl Sensor for Thermo {
    fn read(&self) -> i32 {
        i32::from(self.raw) * 100 / 16
    }

    fn name(&self) -> &'static str {
        "термодатчик"
    }
}

/// Счётчик импульсов.
#[derive(Clone, Copy)]
pub struct Counter {
    pub pulses: u32,
}

impl Sensor for Counter {
    fn read(&self) -> i32 {
        self.pulses as i32
    }
}
// ANCHOR_END: sensor

// ANCHOR: coerce
/// Сумма отсчётов с датчиков разных типов: в срезе лежат толстые
/// указатели одного типа `&dyn Sensor`.
pub fn total(sensors: &[&dyn Sensor]) -> i32 {
    sensors.iter().map(|s| s.read()).sum()
}

pub fn demo() -> i32 {
    let t = Thermo { raw: 344 }; // 21,5 °C
    let c = Counter { pulses: 7 };
    // &Thermo и &Counter приводятся к &dyn Sensor на месте.
    total(&[&t, &c])
}
// ANCHOR_END: coerce

// ANCHOR: call
/// Вызов через таблицу методов: какой `read` позовётся, решает второе
/// слово указателя.
#[unsafe(no_mangle)]
pub fn ch11_read_dyn(s: &dyn Sensor) -> i32 {
    s.read()
}

/// Тот же вызов для известного типа: `read` встроен.
#[unsafe(no_mangle)]
pub fn ch11_read_thermo(t: &Thermo) -> i32 {
    t.read()
}
// ANCHOR_END: call

// ANCHOR: sizes
/// Размеры указателей в словах: `[&Thermo, &dyn Sensor, &[i16],
/// *const dyn Sensor]`, и размер того, на что указывает объект.
pub fn sizes() -> ([usize; 4], usize, usize) {
    let w = size_of::<usize>();
    let t = Thermo { raw: 0 };
    let c = Counter { pulses: 0 };
    let (dt, dc): (&dyn Sensor, &dyn Sensor) = (&t, &c);
    (
        [
            size_of::<&Thermo>() / w,
            size_of::<&dyn Sensor>() / w,
            size_of::<&[i16]>() / w,
            size_of::<*const dyn Sensor>() / w,
        ],
        size_of_val(dt), // 2: размер Thermo из таблицы методов
        size_of_val(dc), // 4: размер Counter
    )
}
// ANCHOR_END: sizes

// ANCHOR: window
/// Датчик поверх чужого буфера: средний отсчёт окна. Объект держит заём.
pub struct Window<'a> {
    pub buf: &'a [i16],
}

impl Sensor for Window<'_> {
    fn read(&self) -> i32 {
        if self.buf.is_empty() {
            return 0;
        }
        let sum: i32 = self.buf.iter().map(|&x| i32::from(x)).sum();
        sum / self.buf.len() as i32
    }
}

/// `&'a dyn Sensor` — это `&'a (dyn Sensor + 'a)`: объект живёт не меньше
/// ссылки на него, и `Window` подходит.
pub fn mean_of<'a>(w: &'a Window<'a>) -> &'a dyn Sensor {
    w
}
// ANCHOR_END: window

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coercion_and_calls() {
        assert_eq!(demo(), 2150 + 7);
        assert_eq!(ch11_read_dyn(&Thermo { raw: -8 }), -50);
        assert_eq!(ch11_read_thermo(&Thermo { raw: 16 }), 100);
        let w = Window { buf: &[1, 2, 6] };
        assert_eq!(mean_of(&w).read(), 3);
        assert_eq!(Window { buf: &[] }.read(), 0);
        assert_eq!((&Counter { pulses: 1 } as &dyn Sensor).name(), "датчик");
        assert_eq!((&Thermo { raw: 1 } as &dyn Sensor).name(), "термодатчик");
    }

    #[test]
    fn fat_pointer_sizes() {
        assert_eq!(sizes(), ([1, 2, 2, 2], 2, 4));
    }
}
