//! Функции, которые глава разбирает в ассемблере (разделы 14.3–14.6 и
//! 14.8): `clone` и `drop` у `Rc` и `Arc`, две загрузки через `&i32` и
//! через `&Cell<i32>`, проверка флага `RefCell`, быстрый путь `LazyLock`,
//! чтение через `Rc<Vec<_>>` и `Rc<[_]>`.

use core::cell::{Cell, RefCell};

#[cfg(feature = "alloc")]
use alloc::{rc::Rc, vec::Vec};

#[cfg(feature = "alloc")]
// ANCHOR: rc_clone
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_rc_clone(r: &Rc<u32>) -> Rc<u32> {
    Rc::clone(r)
}

#[cfg(feature = "alloc")]
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_rc_drop(r: Rc<u32>) {
    drop(r)
}
// ANCHOR_END: rc_clone

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
// ANCHOR: arc_clone
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_arc_clone(r: &alloc::sync::Arc<u32>) -> alloc::sync::Arc<u32> {
    alloc::sync::Arc::clone(r)
}

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_arc_drop(r: alloc::sync::Arc<u32>) {
    drop(r)
}
// ANCHOR_END: arc_clone

// ANCHOR: twice
/// `f` — что угодно: обработчик, функция из другого модуля. Через `&i32`
/// значение измениться не может, и второе чтение не нужно.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_ref_twice(x: &i32, f: fn()) -> i32 {
    let a = *x;
    f();
    a + *x
}

/// Через `&Cell<i32>` может: `f` способна держать ту же ссылку.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_cell_twice(x: &Cell<i32>, f: fn()) -> i32 {
    let a = x.get();
    f();
    a + x.get()
}
// ANCHOR_END: twice

// ANCHOR: refcell
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_cell_inc(x: &Cell<u32>) {
    x.set(x.get() + 1)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_refcell_inc(x: &RefCell<u32>) {
    *x.borrow_mut() += 1
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_refcell_get(x: &RefCell<u32>) -> u32 {
    *x.borrow()
}
// ANCHOR_END: refcell

#[cfg(feature = "std")]
// ANCHOR: lazy_get
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_lazy_crc(data: &[u8]) -> u16 {
    crate::lazy::crc16(data)
}
// ANCHOR_END: lazy_get

#[cfg(feature = "alloc")]
// ANCHOR: rc_vec
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_get_rc_vec(v: &Rc<Vec<i16>>, i: usize) -> i16 {
    v[i]
}

#[cfg(feature = "alloc")]
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch14_get_rc_slice(v: &Rc<[i16]>, i: usize) -> i16 {
    v[i]
}
// ANCHOR_END: rc_vec

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static TOUCHED: Cell<i32> = const { Cell::new(0) };
    }

    #[test]
    fn twice_values() {
        fn nothing() {}
        assert_eq!(ch14_ref_twice(&20, nothing), 40);
        let c = Cell::new(1);
        assert_eq!(ch14_cell_twice(&c, nothing), 2);
        TOUCHED.with(|t| t.set(5));
        // Обработчик, который меняет значение за ячейкой, — законно.
        fn bump() {
            TOUCHED.with(|t| t.set(t.get() + 10));
        }
        let r = TOUCHED.with(|t| ch14_cell_twice(t, bump));
        assert_eq!(r, 5 + 15);
    }

    #[test]
    fn refcell_inc_and_get() {
        let c = RefCell::new(1);
        ch14_refcell_inc(&c);
        assert_eq!(ch14_refcell_get(&c), 2);
        let cell = Cell::new(1);
        ch14_cell_inc(&cell);
        assert_eq!(cell.get(), 2);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn clone_drop_counts() {
        let r = Rc::new(3);
        let c = ch14_rc_clone(&r);
        assert_eq!(Rc::strong_count(&r), 2);
        ch14_rc_drop(c);
        assert_eq!(Rc::strong_count(&r), 1);
        let v = Rc::new(alloc::vec![1i16, 2, 3]);
        let s: Rc<[i16]> = Rc::from(&v[..]);
        assert_eq!((ch14_get_rc_vec(&v, 2), ch14_get_rc_slice(&s, 2)), (3, 3));
    }

    #[test]
    #[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
    fn arc_clone_drop_counts() {
        let a = alloc::sync::Arc::new(3);
        let c = ch14_arc_clone(&a);
        assert_eq!(alloc::sync::Arc::strong_count(&a), 2);
        ch14_arc_drop(c);
        assert_eq!(alloc::sync::Arc::strong_count(&a), 1);
    }
}
