//! Задача D: своя хеш-таблица с открытой адресацией и линейным
//! пробированием, удаление через надгробие. ЗАГОТОВКА: типы и
//! конструкторы даны, тела методов — `todo!()`. Условие — в разделе 15.10.
//!
//! Требования:
//! - `insert_within_capacity` не растит таблицу: нет места (ни пустой
//!   ячейки, ни надгробия на пути) — вернуть пару обратно, как `heapless`;
//! - `insert` при загрузке выше 7/8 перехеширует таблицу, выбрасывая
//!   надгробия;
//! - `get`/`find` идут от домашней ячейки сквозь надгробия до пустой;
//! - `remove` ставит надгробие, а не пустую ячейку (иначе оборвётся
//!   проба ключей за ним);
//! - `probe_len` — длина пробы успешного поиска (ячеек осмотрено, считая
//!   найденную).

#![allow(unused_variables, dead_code, unused_imports)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault, Hash};

/// Детерминированный хешер: воспроизводимая длина пробы.
pub type FixedState = BuildHasherDefault<DefaultHasher>;

// ANCHOR: types
#[derive(Clone)]
enum Slot<K, V> {
    Empty,
    /// Удалённая ячейка: поиск идёт сквозь неё, вставка может занять.
    Tomb,
    Full(K, V),
}

/// Таблица с открытой адресацией; ёмкость — степень двойки.
pub struct OpenTable<K, V, S = FixedState> {
    slots: Vec<Slot<K, V>>,
    live: usize,
    /// Живые плюс надгробия.
    used: usize,
    hasher: S,
}
// ANCHOR_END: types

impl<K: Hash + Eq, V> OpenTable<K, V, FixedState> {
    pub fn with_capacity(cap: usize) -> Self {
        Self::with_capacity_and_hasher(cap, FixedState::default())
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> OpenTable<K, V, S> {
    pub fn with_capacity_and_hasher(cap: usize, hasher: S) -> Self {
        let cap = cap.next_power_of_two().max(8);
        OpenTable { slots: (0..cap).map(|_| Slot::Empty).collect(), live: 0, used: 0, hasher }
    }

    pub fn len(&self) -> usize {
        self.live
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn load(&self) -> f64 {
        self.used as f64 / self.capacity() as f64
    }

    fn home(&self, k: &K) -> usize {
        self.hasher.hash_one(k) as usize & (self.capacity() - 1)
    }

    pub fn insert_within_capacity(&mut self, k: K, v: V) -> Result<Option<V>, (K, V)> {
        todo!()
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        todo!()
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        todo!()
    }

    pub fn probe_len(&self, k: &K) -> Option<usize> {
        todo!()
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        todo!()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.slots.iter().filter_map(|s| match s {
            Slot::Full(k, _) => Some(k),
            _ => None,
        })
    }
}

/// Средняя и наибольшая длина пробы успешного поиска по всем живым ключам.
pub fn probe_stats<K: Hash + Eq, V, S: BuildHasher>(t: &OpenTable<K, V, S>) -> (f64, usize) {
    todo!()
}

/// Таблица ёмкости `cap`, заполненная случайными ключами до доли `load`
/// (без роста), и число реально вставленных ключей.
pub fn filled(cap: usize, load: f64, seed: u64) -> (OpenTable<u64, u64, FixedState>, usize) {
    let mut t = OpenTable::with_capacity(cap);
    let target = (t.capacity() as f64 * load) as usize;
    let mut s = seed | 1;
    let mut inserted = 0;
    while t.len() < target {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        if t.insert_within_capacity(s, s).is_ok() {
            inserted += 1;
        }
    }
    (t, inserted)
}
