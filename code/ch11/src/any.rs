//! Раздел 11.5: `Any` и проверка типа во время работы, приведение к
//! супертрейту.
//!
//! `Any` есть только у типов без заимствований: `TypeId` не различает
//! времена жизни, и `downcast` мог бы продлить заём.
//!
//! ```compile_fail,E0597
//! use core::any::Any;
//! let keep: &dyn Any;
//! {
//!     let local = 5u8;
//!     let r: &u8 = &local;
//!     keep = &r;
//! }
//! assert!(keep.is::<&u8>());
//! ```
//!
//! Обратного приведения, от супертрейта к подтрейту, нет: таблица `dyn
//! Sensor` ничего не знает про `Actuator`.
//!
//! ```compile_fail,E0605
//! use ch11::any::Node;
//! use ch11::fat::Sensor;
//! fn back(s: &dyn Sensor) -> &dyn Node {
//!     s as &dyn Node
//! }
//! ```

use crate::fat::{Sensor, Thermo};
use core::any::Any;

// ANCHOR: node
/// Исполнительное устройство: уставка и её текущее значение.
pub trait Actuator {
    fn set(&self, level: u8) -> u8;
}

/// Узел станции — и датчик, и исполнительное устройство. `Any` в
/// супертрейтах даёт проверку типа во время работы.
pub trait Node: Sensor + Actuator + Any {
    fn id(&self) -> u8;
}

/// Вентилятор с датчиком оборотов.
pub struct Fan {
    pub rpm: u16,
    pub id: u8,
}

impl Sensor for Fan {
    fn read(&self) -> i32 {
        i32::from(self.rpm)
    }

    fn name(&self) -> &'static str {
        "вентилятор"
    }
}

impl Actuator for Fan {
    fn set(&self, level: u8) -> u8 {
        level.min(100)
    }
}

impl Node for Fan {
    fn id(&self) -> u8 {
        self.id
    }
}
// ANCHOR_END: node

// ANCHOR: upcast
/// Объект узла: таблица `dyn Node`.
#[unsafe(no_mangle)]
pub fn ch11_as_node(f: &Fan) -> &dyn Node {
    f
}

/// Приведение к первому супертрейту.
#[unsafe(no_mangle)]
pub fn ch11_upcast_first(n: &dyn Node) -> &dyn Sensor {
    n
}

/// Приведение ко второму супертрейту.
#[unsafe(no_mangle)]
pub fn ch11_upcast_second(n: &dyn Node) -> &dyn Actuator {
    n
}

/// Узел можно передать туда, где ждут датчик: приведение неявное.
pub fn poll_node(n: &dyn Node) -> (i32, u8) {
    (crate::fat::ch11_read_dyn(n), n.set(150))
}
// ANCHOR_END: upcast

// ANCHOR: downcast
/// Проверка типа во время работы: `&dyn Node` → `&dyn Any` → `&Fan`.
#[unsafe(no_mangle)]
pub fn ch11_as_fan(n: &dyn Node) -> Option<&Fan> {
    let any: &dyn Any = n;
    any.downcast_ref::<Fan>()
}

/// Любое значение без заимствований: число, строка, датчик.
pub fn describe(v: &dyn Any) -> &'static str {
    if let Some(&n) = v.downcast_ref::<i32>() {
        if n < 0 { "отрицательное число" } else { "число" }
    } else if v.is::<&str>() {
        "строка"
    } else if v.is::<Thermo>() {
        "термодатчик"
    } else {
        "неизвестно"
    }
}
// ANCHOR_END: downcast

// ANCHOR: box-any
/// `Box<dyn Any>` и сам реализует `Any`, поэтому `b.type_id()` — это
/// `TypeId` коробки, а не значения в ней.
#[cfg(feature = "alloc")]
#[allow(clippy::type_id_on_box)] // ловушку здесь показывают нарочно
pub fn box_ids(b: alloc::boxed::Box<dyn Any>) -> (bool, bool) {
    use core::any::TypeId;
    let of_box = b.type_id() == TypeId::of::<u8>();
    let of_value = (*b).type_id() == TypeId::of::<u8>();
    (of_box, of_value)
}
// ANCHOR_END: box-any

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upcasting() {
        let f = Fan { rpm: 1200, id: 3 };
        let n: &dyn Node = &f;
        assert_eq!(poll_node(n), (1200, 100));
        assert_eq!(ch11_upcast_first(n).name(), "вентилятор");
        assert_eq!(ch11_upcast_second(n).set(42), 42);
        assert_eq!(n.id(), 3);
        // Приведение меняет таблицу, но не адрес данных.
        assert!(core::ptr::addr_eq(ch11_upcast_second(n), n));
    }

    #[test]
    fn downcasting() {
        let f = Fan { rpm: 1, id: 9 };
        assert_eq!(ch11_as_fan(&f).map(|f| f.id), Some(9));
        assert_eq!(describe(&-3), "отрицательное число");
        assert_eq!(describe(&7), "число");
        assert_eq!(describe(&"ок"), "строка");
        assert_eq!(describe(&Thermo { raw: 0 }), "термодатчик");
        assert_eq!(describe(&7u8), "неизвестно");
        assert_eq!(box_ids(Box::new(5u8)), (false, true));
    }
}
