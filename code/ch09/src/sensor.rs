//! Раздел 9.1: определение трейта, реализации, методы по умолчанию и
//! статическая диспетчеризация.

// ANCHOR: trait
/// Источник отсчётов. `read` обязан реализовать каждый тип, `name` и
/// `average` получают реализацию по умолчанию.
pub trait Sensor {
    /// Очередной отсчёт в единицах датчика.
    fn read(&mut self) -> i32;

    /// Имя для журнала.
    fn name(&self) -> &'static str {
        "датчик"
    }

    /// Среднее по `n` отсчётам. Метод по умолчанию вызывает обязательный.
    fn average(&mut self, n: u32) -> i32 {
        let mut sum = 0i64;
        for _ in 0..n {
            sum += i64::from(self.read());
        }
        if n == 0 { 0 } else { (sum / i64::from(n)) as i32 }
    }
}
// ANCHOR_END: trait

// ANCHOR: impls
/// Термодатчик: отсчёт — сотые доли градуса.
pub struct Thermo {
    pub raw: i32,
}

impl Sensor for Thermo {
    fn read(&mut self) -> i32 {
        self.raw
    }

    fn name(&self) -> &'static str {
        "термодатчик"
    }
}

/// Счётчик импульсов: каждый `read` возвращает число импульсов с прошлого.
pub struct Counter {
    pub total: u32,
    pub last: u32,
}

impl Sensor for Counter {
    #[inline(never)]
    fn read(&mut self) -> i32 {
        let d = self.total.wrapping_sub(self.last);
        self.last = self.total;
        d as i32
    }
}
// ANCHOR_END: impls

// ANCHOR: generic
/// Два отсчёта подряд с любого датчика. Для каждого типа `S` компилятор
/// создаёт свою копию функции, и вызов `read` в ней прямой.
pub fn poll_twice<S: Sensor>(s: &mut S) -> i32 {
    s.read() + s.read()
}

#[unsafe(no_mangle)]
pub fn ch09_poll_thermo(t: &mut Thermo) -> i32 {
    poll_twice(t)
}

#[unsafe(no_mangle)]
pub fn ch09_poll_counter(c: &mut Counter) -> i32 {
    poll_twice(c)
}
// ANCHOR_END: generic

// ANCHOR: ufcs
/// Три записи одного вызова. Метод трейта — обычная функция, которой
/// получатель передаётся первым аргументом.
pub fn three_ways(t: &mut Thermo) -> [i32; 3] {
    [t.read(), Sensor::read(t), <Thermo as Sensor>::read(t)]
}
// ANCHOR_END: ufcs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        let mut t = Thermo { raw: 2150 };
        let mut c = Counter { total: 10, last: 4 };
        assert_eq!(t.name(), "термодатчик");
        assert_eq!(c.name(), "датчик");
        assert_eq!(t.average(4), 2150);
        assert_eq!(c.read(), 6);
        assert_eq!(c.read(), 0);
        assert_eq!(c.average(0), 0);
    }

    #[test]
    fn static_dispatch() {
        assert_eq!(ch09_poll_thermo(&mut Thermo { raw: 7 }), 14);
        assert_eq!(ch09_poll_counter(&mut Counter { total: 5, last: 2 }), 3);
        assert_eq!(three_ways(&mut Thermo { raw: 1 }), [1, 1, 1]);
    }
}
