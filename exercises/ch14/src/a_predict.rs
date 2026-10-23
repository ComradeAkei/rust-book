//! Задача A: где паникует `RefCell` и сколько ссылок у `Rc`.
//!
//! Сначала предскажите без компилятора, чем кончится каждый фрагмент,
//! затем сверьте: тесты запускают фрагменты, ловят панику и считают
//! обращения к распределителю. Почему ответ именно такой, объясните в
//! комментарии.

use alloc::rc::{Rc, Weak};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Чем кончился фрагмент с `RefCell`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Доработал до конца.
    Ok,
    /// Паника `RefCell already borrowed`: `borrow_mut`, пока живо другое
    /// заимствование.
    AlreadyBorrowed,
    /// Паника `RefCell already mutably borrowed`: `borrow`, пока жив
    /// `borrow_mut`.
    AlreadyMutablyBorrowed,
}

// ANCHOR: refcell
/// Шесть фрагментов над списком каналов `[1, 2, 3]`.
pub const REFCELL: [fn(&RefCell<Vec<u8>>); 6] = [
    |c| {
        let first = c.borrow()[0];
        c.borrow_mut().push(first);
    },
    |c| {
        let (n, cap) = (c.borrow().len(), c.borrow_mut().capacity());
        assert!(n <= cap);
    },
    |c| c.borrow_mut().push(c.borrow()[0]),
    |c| {
        if c.borrow().len() < 4 {
            c.borrow_mut().push(0);
        }
    },
    |c| match c.borrow().len() {
        n if n < 4 => c.borrow_mut().push(0),
        _ => {}
    },
    |c| {
        if let Some(&last) = c.borrow().last() {
            c.borrow_mut().push(last);
        }
    },
];
// ANCHOR_END: refcell

// ANCHOR: rc
/// Шесть фрагментов с `Rc`. Каждый возвращает `(strong, weak)` — то, что
/// показывают `strong_count` и `weak_count` в его последней строке.
pub const RC: [fn() -> (usize, usize); 6] = [
    || {
        let a = Rc::new(String::from("adc0"));
        let b = Rc::clone(&a);
        let _copy: String = (*a).clone();
        let _w = Rc::downgrade(&b);
        drop(b);
        (Rc::strong_count(&a), Rc::weak_count(&a))
    },
    || {
        let v = vec![1u8, 2, 3];
        let r: Rc<[u8]> = Rc::from(v);
        let _r2 = Rc::clone(&r);
        (Rc::strong_count(&r), Rc::weak_count(&r))
    },
    || {
        let mut a = Rc::new(vec![1u32]);
        let _b = Rc::clone(&a);
        Rc::make_mut(&mut a).push(2);
        (Rc::strong_count(&a), Rc::weak_count(&a))
    },
    || {
        let a = Rc::new(5u32);
        let w = Rc::downgrade(&a);
        let _w2 = w.clone();
        drop(a);
        (w.strong_count(), w.weak_count())
    },
    || {
        struct Node {
            _me: Weak<Node>,
        }
        let a = Rc::new_cyclic(|me| Node { _me: me.clone() });
        (Rc::strong_count(&a), Rc::weak_count(&a))
    },
    || {
        let log = Rc::new(RefCell::new(Vec::new()));
        let push = {
            let log = Rc::clone(&log);
            move |x: u32| log.borrow_mut().push(x)
        };
        push(1);
        push(2);
        (Rc::strong_count(&log), Rc::weak_count(&log))
    },
];
// ANCHOR_END: rc

// ANCHOR: predicted
/// Чем кончатся фрагменты `REFCELL`.
pub const fn predicted_refcell() -> [Outcome; 6] {
    todo!()
}

/// Для фрагментов `RC`: сколько раз фрагмент просит память у
/// распределителя (`alloc` и `realloc`), `strong` и `weak` в конце.
pub const fn predicted_rc() -> [(usize, usize, usize); 6] {
    todo!()
}
// ANCHOR_END: predicted
