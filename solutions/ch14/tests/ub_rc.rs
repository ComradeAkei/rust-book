//! Приложение E, задача B: ошибка первой версии решения, которую нашёл
//! Miri. Счётчик со ссылкой на весь блок: `inner()` возвращает `&Inner<T>`,
//! и эта ссылка накрывает значение. Пока `drop` уничтожает значение через
//! `&mut`, деструктор значения роняет слабую ссылку на тот же блок, она
//! берёт `&Inner<T>` — и две ссылки пересекаются. Нативно всё работает;
//! Stacked Borrows сообщает о неопределённом поведении, Tree Borrows
//! (`MIRIFLAGS=-Zmiri-tree-borrows`) такой код пропускает. Модель
//! заимствований ещё не утверждена, поэтому решение пишется так, чтобы
//! его принимали обе, как `Rc` в `std`. Запуск:
//!
//! ```console
//! $ cargo +nightly miri test -p ch14-solutions --test ub_rc -- --ignored
//! ```

use std::cell::{Cell, RefCell};
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

struct Inner<T> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: ManuallyDrop<T>,
}

struct Strong<T>(NonNull<Inner<T>>);
struct Weak<T>(NonNull<Inner<T>>);

// ANCHOR: whole
impl<T> Strong<T> {
    fn new(value: T) -> Self {
        let inner =
            Inner { strong: Cell::new(1), weak: Cell::new(1), value: ManuallyDrop::new(value) };
        Self(NonNull::from(Box::leak(Box::new(inner))))
    }

    /// Ссылка на весь блок — вместе со значением.
    fn inner(&self) -> &Inner<T> {
        // SAFETY: блок жив, пока жива сильная ссылка.
        unsafe { self.0.as_ref() }
    }

    fn downgrade(&self) -> Weak<T> {
        self.inner().weak.set(self.inner().weak.get() + 1);
        Weak(self.0)
    }
}

impl<T> Drop for Strong<T> {
    fn drop(&mut self) {
        let strong = &self.inner().strong;
        strong.set(strong.get() - 1);
        if strong.get() == 0 {
            // SAFETY: последняя сильная ссылка.
            unsafe { ManuallyDrop::drop(&mut (*self.0.as_ptr()).value) };
            drop(Weak(self.0));
        }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        // SAFETY: блок жив, пока жива слабая ссылка.
        let inner = unsafe { self.0.as_ref() };
        inner.weak.set(inner.weak.get() - 1);
        if inner.weak.get() == 0 {
            // SAFETY: последняя ссылка на блок.
            drop(unsafe { Box::from_raw(self.0.as_ptr()) });
        }
    }
}
// ANCHOR_END: whole

struct Node {
    parent: RefCell<Option<Weak<Node>>>,
    children: RefCell<Vec<Strong<Node>>>,
    /// Байты вне `UnsafeCell`: только на них общая ссылка `&Inner<T>`
    /// даёт право лишь на чтение, и конфликт с `&mut` виден Miri.
    _id: u32,
}

// ANCHOR: tree
#[test]
#[ignore = "UB: запускать под Miri"]
fn child_drops_weak_to_dying_parent() {
    let node = || {
        Strong::new(Node { parent: RefCell::new(None), children: RefCell::new(Vec::new()), _id: 0 })
    };
    let root = node();
    let child = node();
    *child.inner().value.parent.borrow_mut() = Some(root.downgrade());
    root.inner().value.children.borrow_mut().push(child);
    drop(root); // значение корня уничтожает ребёнка, а он — Weak на корень
}
// ANCHOR_END: tree
