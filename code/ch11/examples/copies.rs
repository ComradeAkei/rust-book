//! Раздел 11.2: две таблицы для одной пары «тип — трейт».
//!
//! Обобщённая функция мономорфизуется отдельно, и в отладочной сборке её
//! копия может попасть в другую единицу кодогенерации со своей копией
//! таблицы. Адрес данных у обоих указателей один, а сравнение толстых
//! указателей целиком смотрит и на таблицу.
//!
//! Запуск: `cargo run -p ch11 --example copies`, затем то же с `--release`.
use ch11::fat::{Sensor, Thermo};
use std::ptr;

fn as_dyn<T: Sensor>(t: &T) -> &dyn Sensor {
    t
}

fn main() {
    let t = Thermo { raw: 344 };
    let here: &dyn Sensor = &t;
    let there = as_dyn(&t);
    println!("ptr::addr_eq: {}", ptr::addr_eq(here, there));
    println!("ptr::eq:      {}", ptr::eq(here, there));
    assert_eq!(here.read(), there.read());
}
