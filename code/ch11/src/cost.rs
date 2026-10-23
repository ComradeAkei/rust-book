//! Раздел 11.6: цена косвенного вызова и девиртуализация.

use crate::fat::{Sensor, Thermo};
use crate::vtable::SENSORS;

// ANCHOR: devirt
/// Тип известен в той же функции: LLVM видит, какая таблица лежит в
/// указателе, вызов становится прямым и встраивается.
#[unsafe(no_mangle)]
pub fn ch11_devirt_local() -> i32 {
    let t = Thermo { raw: 344 };
    let s: &dyn Sensor = &t;
    s.read()
}

/// Объекты и таблицы — константы: адреса методов LLVM прочтёт из них
/// при компиляции.
#[unsafe(no_mangle)]
pub fn ch11_devirt_static() -> i32 {
    SENSORS.iter().map(|s| s.read()).sum()
}

/// `black_box` прячет указатель от оптимизатора, как его прячет граница
/// крейта или массив, заполненный во время работы.
#[unsafe(no_mangle)]
pub fn ch11_devirt_hidden() -> i32 {
    let t = Thermo { raw: 344 };
    let s: &dyn Sensor = core::hint::black_box(&t);
    s.read()
}
// ANCHOR_END: devirt

// ANCHOR: poll
/// Объекты пришли снаружи: в цикле остаётся косвенный вызов.
#[unsafe(no_mangle)]
pub fn ch11_poll_dyn(sensors: &[&dyn Sensor]) -> i32 {
    let mut sum = 0i32;
    for s in sensors {
        sum = sum.wrapping_add(s.read());
    }
    sum
}
// ANCHOR_END: poll

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fat::Counter;

    #[test]
    fn same_results() {
        assert_eq!(ch11_devirt_local(), 2150);
        assert_eq!(ch11_devirt_hidden(), 2150);
        assert_eq!(ch11_devirt_static(), 2162);
        let c = Counter { pulses: u32::MAX };
        assert_eq!(ch11_poll_dyn(&[&c, &Thermo { raw: 16 }]), 99);
    }
}
