//! Задача C: двусвязный список дважды — на `Rc`, `Weak` и `RefCell` и на
//! арене с индексами.
//!
//! Оба списка дают одно и то же: вставку в оба конца, снятие с обоих
//! концов и удаление по ручке за O(1) — то, ради чего двусвязные списки
//! и пишут, например для кэша LRU. Ручка на удалённый узел ничего не
//! ломает: `remove` возвращает `None`.

use alloc::rc::{Rc, Weak};
use alloc::vec::Vec;
use core::cell::RefCell;

// ANCHOR: rc_types
type Link<T> = Rc<RefCell<RcNode<T>>>;

struct RcNode<T> {
    value: Option<T>,
    /// Сильная ссылка вперёд: узлом владеет предыдущий.
    next: Option<Link<T>>,
    /// Слабая назад: иначе соседи держали бы друг друга вечно.
    prev: Weak<RefCell<RcNode<T>>>,
}

pub struct RcList<T> {
    head: Option<Link<T>>,
    tail: Weak<RefCell<RcNode<T>>>,
    len: usize,
}

/// Ручка не продлевает жизнь узла.
pub struct RcHandle<T>(Weak<RefCell<RcNode<T>>>);
// ANCHOR_END: rc_types

impl<T> Clone for RcHandle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for RcList<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ANCHOR: rc_impl
impl<T> RcList<T> {
    pub fn new() -> Self {
        Self { head: None, tail: Weak::new(), len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn node(value: T) -> Link<T> {
        Rc::new(RefCell::new(RcNode { value: Some(value), next: None, prev: Weak::new() }))
    }

    pub fn push_front(&mut self, value: T) -> RcHandle<T> {
        let n = Self::node(value);
        match self.head.take() {
            Some(old) => {
                old.borrow_mut().prev = Rc::downgrade(&n);
                n.borrow_mut().next = Some(old);
            }
            None => self.tail = Rc::downgrade(&n),
        }
        let h = RcHandle(Rc::downgrade(&n));
        self.head = Some(n);
        self.len += 1;
        h
    }

    pub fn push_back(&mut self, value: T) -> RcHandle<T> {
        let n = Self::node(value);
        let h = RcHandle(Rc::downgrade(&n));
        match self.tail.upgrade() {
            Some(old) => {
                n.borrow_mut().prev = Rc::downgrade(&old);
                self.tail = Rc::downgrade(&n);
                old.borrow_mut().next = Some(n);
            }
            None => {
                self.tail = Rc::downgrade(&n);
                self.head = Some(n);
            }
        }
        self.len += 1;
        h
    }

    /// Вынимает узел из цепочки и отдаёт значение.
    fn unlink(&mut self, n: &Link<T>) -> Option<T> {
        let mut node = n.borrow_mut();
        let value = node.value.take()?;
        let next = node.next.take();
        let prev = core::mem::take(&mut node.prev).upgrade();
        drop(node);
        match &next {
            Some(nx) => nx.borrow_mut().prev = prev.as_ref().map_or_else(Weak::new, Rc::downgrade),
            None => self.tail = prev.as_ref().map_or_else(Weak::new, Rc::downgrade),
        }
        match prev {
            Some(p) => p.borrow_mut().next = next,
            None => self.head = next,
        }
        self.len -= 1;
        Some(value)
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let n = self.head.clone()?;
        self.unlink(&n)
    }

    pub fn pop_back(&mut self) -> Option<T> {
        let n = self.tail.upgrade()?;
        self.unlink(&n)
    }

    pub fn remove(&mut self, h: &RcHandle<T>) -> Option<T> {
        let n = h.0.upgrade()?;
        self.unlink(&n)
    }

    /// Переставить узел в начало — шаг кэша LRU. Ручка остаётся верной.
    pub fn move_to_front(&mut self, h: &RcHandle<T>) -> bool {
        let Some(n) = h.0.upgrade() else { return false };
        if self.head.as_ref().is_some_and(|hd| Rc::ptr_eq(hd, &n)) {
            return true;
        }
        let Some(value) = self.unlink(&n) else { return false };
        n.borrow_mut().value = Some(value);
        if let Some(old) = self.head.take() {
            old.borrow_mut().prev = Rc::downgrade(&n);
            n.borrow_mut().next = Some(old);
        } else {
            self.tail = Rc::downgrade(&n);
        }
        self.head = Some(n);
        self.len += 1;
        true
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        let mut out = Vec::with_capacity(self.len);
        let mut cur = self.head.clone();
        while let Some(n) = cur {
            let node = n.borrow();
            out.extend(node.value.clone());
            cur = node.next.clone();
        }
        out
    }
}

impl<T> Drop for RcList<T> {
    /// Без этого длинный список уничтожался бы рекурсивно — узел за узлом
    /// через `next` — и переполнил бы стек.
    fn drop(&mut self) {
        let mut cur = self.head.take();
        while let Some(n) = cur {
            cur = n.borrow_mut().next.take();
        }
    }
}
// ANCHOR_END: rc_impl

// ANCHOR: arena_types
const NIL: u32 = u32::MAX;

struct Slot<T> {
    value: Option<T>,
    prev: u32,
    next: u32,
    /// Поколение: растёт при каждом освобождении слота, и старые ручки
    /// перестают совпадать.
    generation: u32,
}

pub struct ArenaList<T> {
    slots: Vec<Slot<T>>,
    head: u32,
    tail: u32,
    /// Список свободных слотов через `next`.
    free: u32,
    len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    index: u32,
    generation: u32,
}
// ANCHOR_END: arena_types

impl<T> Default for ArenaList<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ANCHOR: arena_impl
impl<T> ArenaList<T> {
    pub fn new() -> Self {
        Self { slots: Vec::new(), head: NIL, tail: NIL, free: NIL, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn alloc(&mut self, value: T) -> u32 {
        if self.free != NIL {
            let i = self.free;
            let s = &mut self.slots[i as usize];
            self.free = s.next;
            s.value = Some(value);
            i
        } else {
            let i =
                u32::try_from(self.slots.len()).ok().filter(|&i| i != NIL).expect("арена полна");
            self.slots.push(Slot { value: Some(value), prev: NIL, next: NIL, generation: 0 });
            i
        }
    }

    fn handle(&self, i: u32) -> Handle {
        Handle { index: i, generation: self.slots[i as usize].generation }
    }

    fn link_front(&mut self, i: u32) {
        self.slots[i as usize].prev = NIL;
        self.slots[i as usize].next = self.head;
        match self.head {
            NIL => self.tail = i,
            h => self.slots[h as usize].prev = i,
        }
        self.head = i;
        self.len += 1;
    }

    pub fn push_front(&mut self, value: T) -> Handle {
        let i = self.alloc(value);
        self.link_front(i);
        self.handle(i)
    }

    pub fn push_back(&mut self, value: T) -> Handle {
        let i = self.alloc(value);
        self.slots[i as usize].next = NIL;
        self.slots[i as usize].prev = self.tail;
        match self.tail {
            NIL => self.head = i,
            t => self.slots[t as usize].next = i,
        }
        self.tail = i;
        self.len += 1;
        self.handle(i)
    }

    fn unlink(&mut self, i: u32) {
        let Slot { prev, next, .. } = self.slots[i as usize];
        match prev {
            NIL => self.head = next,
            p => self.slots[p as usize].next = next,
        }
        match next {
            NIL => self.tail = prev,
            n => self.slots[n as usize].prev = prev,
        }
        self.len -= 1;
    }

    fn release(&mut self, i: u32) -> T {
        self.unlink(i);
        let s = &mut self.slots[i as usize];
        s.generation = s.generation.wrapping_add(1);
        s.next = self.free;
        self.free = i;
        s.value.take().unwrap()
    }

    fn live(&self, h: Handle) -> bool {
        self.slots
            .get(h.index as usize)
            .is_some_and(|s| s.generation == h.generation && s.value.is_some())
    }

    pub fn pop_front(&mut self) -> Option<T> {
        (self.head != NIL).then(|| self.release(self.head))
    }

    pub fn pop_back(&mut self) -> Option<T> {
        (self.tail != NIL).then(|| self.release(self.tail))
    }

    pub fn remove(&mut self, h: Handle) -> Option<T> {
        self.live(h).then(|| self.release(h.index))
    }

    pub fn move_to_front(&mut self, h: Handle) -> bool {
        if !self.live(h) {
            return false;
        }
        if self.head != h.index {
            self.unlink(h.index);
            self.link_front(h.index);
        }
        true
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        let mut out = Vec::with_capacity(self.len);
        let mut i = self.head;
        while i != NIL {
            let s = &self.slots[i as usize];
            out.extend(s.value.clone());
            i = s.next;
        }
        out
    }
}
// ANCHOR_END: arena_impl
