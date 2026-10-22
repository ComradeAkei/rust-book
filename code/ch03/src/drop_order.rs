//! Раздел 3.2: `Drop` и порядок уничтожения.
//!
//! Журнал событий — вектор в thread-local переменной: каждый тест идёт в
//! своём потоке и видит только свои события. Деструктор `Noisy` пишет в
//! журнал своё имя.
//!
//! Метод `Drop::drop` нельзя вызвать руками: иначе компилятор вызвал бы
//! его второй раз в конце области видимости.
//!
//! ```compile_fail,E0040
//! struct Port;
//! impl Drop for Port {
//!     fn drop(&mut self) {}
//! }
//!
//! let mut p = Port;
//! p.drop();
//! ```

use std::cell::RefCell;
use std::vec;
use std::vec::Vec;

// ANCHOR: noisy
std::thread_local! {
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// Записать событие в журнал текущего потока.
pub fn log(event: &'static str) {
    LOG.with_borrow_mut(|l| l.push(event));
}

/// Забрать журнал текущего потока, оставив на его месте пустой.
pub fn take_log() -> Vec<&'static str> {
    LOG.with_borrow_mut(core::mem::take)
}

/// Значение, которое при уничтожении пишет в журнал своё имя.
pub struct Noisy(pub &'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        log(self.0);
    }
}
// ANCHOR_END: noisy

// ANCHOR: locals
/// Локальные переменные уничтожаются в порядке, обратном объявлению.
pub fn locals() {
    let _a = Noisy("a");
    let _b = Noisy("b");
    log("конец блока");
}
// ANCHOR_END: locals

// ANCHOR: fields
pub struct Sensor {
    pub bus: Noisy,
    pub irq: Noisy,
}

/// Сначала выполняется `drop` самой структуры, затем поля в порядке
/// объявления.
impl Drop for Sensor {
    fn drop(&mut self) {
        log("Sensor::drop");
    }
}

pub fn fields() {
    let _s = Sensor { irq: Noisy("irq"), bus: Noisy("bus") };
}
// ANCHOR_END: fields

// ANCHOR: collections
/// Элементы кортежей, массивов и векторов уничтожаются по порядку, от
/// первого к последнему.
#[allow(clippy::useless_vec)] // вектор нарочно: элементы лежат в куче
pub fn collections() {
    let _t = (Noisy("t.0"), Noisy("t.1"));
    let _v = vec![Noisy("v[0]"), Noisy("v[1]")];
}
// ANCHOR_END: collections

// ANCHOR: reassign
/// Присваивание уничтожает старое значение, затенение — нет.
#[allow(unused_assignments)] // старое значение `x` нужно только ради деструктора
pub fn reassign_and_shadow() {
    let mut x = Noisy("x старый");
    x = Noisy("x новый"); // старое значение уничтожено здесь
    log("после присваивания");
    let _y = Noisy("y внешний");
    let _y = Noisy("y внутренний"); // внешний y жив до конца блока
    log("после затенения");
    let _ = &x;
}
// ANCHOR_END: reassign

// ANCHOR: unwinding
/// При панике стек раскручивается, и деструкторы локальных переменных
/// выполняются в том же порядке, что и при обычном выходе.
pub fn unwinding(fail: bool) {
    let _a = Noisy("a");
    let _b = Noisy("b");
    if fail {
        panic!("отказ датчика");
    }
    log("без паники");
}
// ANCHOR_END: unwinding

// ANCHOR: partial_array
fn boom() -> Noisy {
    panic!("второй элемент не создан");
}

/// Паника посреди создания массива: уничтожаются только уже готовые
/// элементы.
pub fn partial_array() {
    let _arr = [Noisy("arr[0]"), boom(), Noisy("arr[2]")];
}
// ANCHOR_END: partial_array

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::catch_unwind;

    // ANCHOR: tests
    #[test]
    fn locals_reverse() {
        locals();
        assert_eq!(take_log(), ["конец блока", "b", "a"]);
    }

    #[test]
    fn fields_forward_after_own_drop() {
        fields();
        assert_eq!(take_log(), ["Sensor::drop", "bus", "irq"]);
    }

    #[test]
    fn elements_forward() {
        collections();
        assert_eq!(take_log(), ["v[0]", "v[1]", "t.0", "t.1"]);
    }

    #[test]
    fn assignment_drops_old_value() {
        reassign_and_shadow();
        assert_eq!(
            take_log(),
            [
                "x старый",
                "после присваивания",
                "после затенения",
                "y внутренний",
                "y внешний",
                "x новый",
            ]
        );
    }
    // ANCHOR_END: tests

    #[test]
    fn unwinding_runs_destructors() {
        unwinding(false);
        assert_eq!(take_log(), ["без паники", "b", "a"]);
        assert!(catch_unwind(|| unwinding(true)).is_err());
        assert_eq!(take_log(), ["b", "a"]);
    }

    #[test]
    fn partial_array_drops_finished_elements() {
        assert!(catch_unwind(partial_array).is_err());
        assert_eq!(take_log(), ["arr[0]"]);
    }
}
