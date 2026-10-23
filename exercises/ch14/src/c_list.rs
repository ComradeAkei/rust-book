//! Задача C: двусвязный список дважды — на `Rc`, `Weak` и `RefCell` и на
//! арене с индексами.
//!
//! Оба списка дают одно и то же: вставку в оба конца, снятие с обоих
//! концов и удаление по ручке за O(1) — то, ради чего двусвязные списки
//! и пишут, например для кэша LRU. Ручка на удалённый узел ничего не
//! должна ломать: `remove` возвращает `None`, `move_to_front` — `false`.
//!
//! Типы даны, тела — `todo!()`. Тест `task_c_long_rc_list_drops_without_recursion`
//! уничтожает список из миллиона узлов: посмотрите, что при этом сделает
//! `RcList` без вашей помощи.

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

impl<T> RcList<T> {
    pub fn new() -> Self {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push_front(&mut self, value: T) -> RcHandle<T> {
        todo!()
    }

    pub fn push_back(&mut self, value: T) -> RcHandle<T> {
        todo!()
    }

    pub fn pop_front(&mut self) -> Option<T> {
        todo!()
    }

    pub fn pop_back(&mut self) -> Option<T> {
        todo!()
    }

    pub fn remove(&mut self, h: &RcHandle<T>) -> Option<T> {
        todo!()
    }

    /// Переставить узел в начало — шаг кэша LRU. Ручка остаётся верной.
    pub fn move_to_front(&mut self, h: &RcHandle<T>) -> bool {
        todo!()
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        todo!()
    }
}

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

impl<T> ArenaList<T> {
    pub fn new() -> Self {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push_front(&mut self, value: T) -> Handle {
        todo!()
    }

    pub fn push_back(&mut self, value: T) -> Handle {
        todo!()
    }

    pub fn pop_front(&mut self) -> Option<T> {
        todo!()
    }

    pub fn pop_back(&mut self) -> Option<T> {
        todo!()
    }

    pub fn remove(&mut self, h: Handle) -> Option<T> {
        todo!()
    }

    /// Переставить узел в начало — шаг кэша LRU. Ручка остаётся верной.
    pub fn move_to_front(&mut self, h: Handle) -> bool {
        todo!()
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        todo!()
    }
}
