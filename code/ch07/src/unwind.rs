//! Раздел 7.6: посадочные площадки. Деструктор локальной переменной
//! должен выполниться и при обычном возврате, и при раскрутке паники.

use core::cell::Cell;

// ANCHOR: guarded
/// Считает, сколько раз его уничтожили.
pub struct Count<'a>(pub &'a Cell<u32>);

impl Drop for Count<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

/// Вызов, который может паниковать, при живом `Count`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_guarded(drops: &Cell<u32>, f: fn(u32) -> u32, x: u32) -> u32 {
    let _g = Count(drops);
    f(x)
}

/// То же, но вызываемая функция объявлена `extern "C"`: такая функция
/// раскрутку наружу не выпускает, и площадка не нужна.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_guarded_nounwind(drops: &Cell<u32>, f: extern "C" fn(u32) -> u32, x: u32) -> u32 {
    let _g = Count(drops);
    f(x)
}
// ANCHOR_END: guarded

#[cfg(test)]
mod tests {
    use super::*;

    fn double(x: u32) -> u32 {
        x * 2
    }

    extern "C" fn triple(x: u32) -> u32 {
        x * 3
    }

    fn explode(_: u32) -> u32 {
        panic!("отказ датчика")
    }

    #[test]
    fn drop_runs_on_return() {
        let c = Cell::new(0);
        assert_eq!(ch07_guarded(&c, double, 21), 42);
        assert_eq!(ch07_guarded_nounwind(&c, triple, 3), 9);
        assert_eq!(c.get(), 2);
    }

    #[test]
    fn drop_runs_on_unwind() {
        let c = Cell::new(0);
        // `&Cell` не `UnwindSafe`: после паники Cell мог остаться в
        // промежуточном состоянии. Здесь мы проверяем ровно это состояние.
        let r =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ch07_guarded(&c, explode, 1)));
        assert!(r.is_err());
        assert_eq!(c.get(), 1, "посадочная площадка вызвала drop");
    }
}
