//! Раздел 3.6: деструктор не гарантирован.
//!
//! ```text
//! cargo run -p ch03 --example leaks
//! ```
//!
//! Счётчик живых объектов растёт при создании и уменьшается в `drop`. Два
//! способа оставить объект живым навсегда без единой строки `unsafe`:
//! `mem::forget` и цикл из `Rc`.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

static ALIVE: AtomicUsize = AtomicUsize::new(0);

// ANCHOR: leaks
struct Node {
    next: RefCell<Option<Rc<Node>>>,
}

impl Node {
    fn new() -> Rc<Node> {
        ALIVE.fetch_add(1, Relaxed);
        Rc::new(Node { next: RefCell::new(None) })
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        ALIVE.fetch_sub(1, Relaxed);
    }
}

fn main() {
    {
        let a = Node::new();
        let b = Node::new();
        *a.next.borrow_mut() = Some(b.clone());
        *b.next.borrow_mut() = Some(a.clone()); // a → b → a: цикл
    } // счётчики ссылок упали с 2 до 1, но не до 0
    println!("после цикла Rc живы: {}", ALIVE.load(Relaxed));

    std::mem::forget(Node::new());
    println!("после mem::forget живы: {}", ALIVE.load(Relaxed));

    {
        let _c = Node::new();
    }
    println!("после обычного блока живы: {}", ALIVE.load(Relaxed));
}
// ANCHOR_END: leaks
