//! Раздел 3.3: флаги drop и частичные перемещения.
//!
//! Нельзя переместить поле из значения, у которого есть свой деструктор:
//! `Drop::drop` получил бы структуру с дырой.
//!
//! ```compile_fail,E0509
//! struct Packet {
//!     header: String,
//!     payload: Vec<u8>,
//! }
//! impl Drop for Packet {
//!     fn drop(&mut self) {}
//! }
//!
//! fn header(p: Packet) -> String {
//!     p.header
//! }
//! ```
//!
//! Из-под ссылки и из элемента вектора по индексу переместить тоже нельзя:
//! владелец остался бы с дырой, о которой не знает.
//!
//! ```compile_fail,E0507
//! let frames = vec![String::from("a"), String::from("b")];
//! let first = frames[0];
//! ```
//!
//! После частичного перемещения структурой целиком пользоваться нельзя,
//! а оставшимися полями — можно:
//!
//! ```compile_fail,E0382
//! struct Packet {
//!     header: String,
//!     payload: Vec<u8>,
//! }
//!
//! let p = Packet { header: String::from("h"), payload: vec![1] };
//! let h = p.header;
//! let len = p.payload.len(); // так можно
//! let q = p; // а так нет: `p.header` уже перемещён
//! ```

use crate::drop_order::{Noisy, log};

// ANCHOR: partial
pub struct Packet {
    pub header: Noisy,
    pub payload: Noisy,
}

/// Поле можно забрать из структуры по одному. Оставшееся поле
/// уничтожается в конце области видимости `p`, а забранное — там, где
/// кончается его новый владелец.
pub fn partial_move() {
    let p = Packet { header: Noisy("header"), payload: Noisy("payload") };
    let h = p.header;
    log("header забран");
    drop(h);
    log("конец функции");
}
// ANCHOR_END: partial

// ANCHOR: update
pub struct Config {
    pub name: Noisy,
    pub port: Noisy,
}

/// Синтаксис обновления `..base` перемещает недостающие поля из `base`.
/// Поле `name` в `base` остаётся и уничтожается вместе с остатком `base`.
pub fn struct_update() {
    let base = Config { name: Noisy("base.name"), port: Noisy("base.port") };
    let _cfg = Config { name: Noisy("cfg.name"), ..base };
    log("cfg собран");
}
// ANCHOR_END: update

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop_order::take_log;

    #[test]
    fn partial_move_order() {
        partial_move();
        assert_eq!(take_log(), ["header забран", "header", "конец функции", "payload"]);
    }

    #[test]
    fn struct_update_order() {
        struct_update();
        assert_eq!(take_log(), ["cfg собран", "cfg.name", "base.port", "base.name"]);
    }
}
