//! Задача D: своя хеш-таблица с открытой адресацией и линейным
//! пробированием, с удалением через надгробие (tombstone). Цель —
//! измерить, как средняя длина пробы растёт с коэффициентом загрузки и
//! как её удлиняют надгробия, и понять, почему SwissTable из `std`
//! держит пробу короткой.
//!
//! Это та же схема, что в самодельной `table.c` из вступления главы, но
//! на безопасном Rust: `get` возвращает `Option<&V>` (заём, который не
//! переживёт вставку, — E0502, а не висячий указатель), а ёмкость —
//! жёсткая: таблица без места возвращает `Err`, а не портит память.

use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault, Hash};

/// Детерминированный хешер: `DefaultHasher` с фиксированным ключом.
/// Нужен, чтобы длина пробы была воспроизводима от запуска к запуску —
/// в отличие от `RandomState`, который и защищает `std::HashMap`.
pub type FixedState = BuildHasherDefault<DefaultHasher>;

// ANCHOR: slot
#[derive(Clone)]
enum Slot<K, V> {
    Empty,
    /// Удалённая ячейка: поиск идёт сквозь неё, вставка может занять.
    Tomb,
    Full(K, V),
}
// ANCHOR_END: slot

// ANCHOR: table
/// Таблица с открытой адресацией. Ёмкость (`slots.len()`) — степень
/// двойки, поэтому индекс берётся маской, а не остатком.
pub struct OpenTable<K, V, S = FixedState> {
    slots: Vec<Slot<K, V>>,
    live: usize,
    /// Живые плюс надгробия: именно это число ограничивает поиск.
    used: usize,
    hasher: S,
}
// ANCHOR_END: table

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

    /// Коэффициент загрузки с учётом надгробий.
    pub fn load(&self) -> f64 {
        self.used as f64 / self.capacity() as f64
    }

    fn home(&self, k: &K) -> usize {
        self.hasher.hash_one(k) as usize & (self.capacity() - 1)
    }

    // ANCHOR: insert_within
    /// Вставка без роста: если места нет (ни пустой ячейки, ни надгробия
    /// на пути), вернуть пару обратно, как `heapless`. Так таблицу можно
    /// довести до любой загрузки и измерить пробу, не дав ей перехешиться.
    pub fn insert_within_capacity(&mut self, k: K, v: V) -> Result<Option<V>, (K, V)> {
        // Что в осмотренной ячейке — отдельным шагом, чтобы заём `slots[i]`
        // кончился до изменения таблицы.
        enum Step {
            Found,
            Empty,
            Tomb,
            Occupied,
        }
        let mask = self.capacity() - 1;
        let mut i = self.home(&k);
        let mut first_tomb = None;
        for _ in 0..self.capacity() {
            let step = match &self.slots[i] {
                Slot::Full(ek, _) if *ek == k => Step::Found,
                Slot::Full(..) => Step::Occupied,
                Slot::Tomb => Step::Tomb,
                Slot::Empty => Step::Empty,
            };
            match step {
                Step::Found => {
                    let Slot::Full(_, ev) = &mut self.slots[i] else { unreachable!() };
                    return Ok(Some(std::mem::replace(ev, v)));
                }
                Step::Empty => {
                    let at = first_tomb.unwrap_or(i);
                    if matches!(self.slots[at], Slot::Empty) {
                        self.used += 1;
                    }
                    self.slots[at] = Slot::Full(k, v);
                    self.live += 1;
                    return Ok(None);
                }
                Step::Tomb => {
                    first_tomb.get_or_insert(i);
                }
                Step::Occupied => {}
            }
            i = (i + 1) & mask;
        }
        // Ни пустой ячейки, ни надгробия: занять надгробие, если было.
        if let Some(at) = first_tomb {
            self.slots[at] = Slot::Full(k, v);
            self.live += 1;
            Ok(None)
        } else {
            Err((k, v))
        }
    }
    // ANCHOR_END: insert_within

    // ANCHOR: insert
    /// Вставка с ростом: при загрузке выше 7/8 таблица перехешивается
    /// (и заодно выбрасывает надгробия), как `std::HashMap`.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if (self.used + 1) * 8 > self.capacity() * 7 {
            self.grow();
        }
        match self.insert_within_capacity(k, v) {
            Ok(prev) => prev,
            Err(_) => unreachable!("после grow место есть"),
        }
    }

    fn grow(&mut self) {
        // Много надгробий — хватит очистки без увеличения; иначе вдвое.
        let cap = if self.live * 8 >= self.capacity() * 7 {
            self.capacity() * 2
        } else {
            self.capacity()
        };
        let old = std::mem::replace(&mut self.slots, (0..cap).map(|_| Slot::Empty).collect());
        self.live = 0;
        self.used = 0;
        for slot in old {
            if let Slot::Full(k, v) = slot {
                let _ = self.insert_within_capacity(k, v);
            }
        }
    }
    // ANCHOR_END: insert

    // ANCHOR: get
    /// Поиск идёт от домашней ячейки, сквозь надгробия, до пустой —
    /// пустая ячейка означает, что дальше ключа нет.
    pub fn get(&self, k: &K) -> Option<&V> {
        self.find(k).map(|(i, _)| {
            let Slot::Full(_, v) = &self.slots[i] else { unreachable!() };
            v
        })
    }

    /// Ячейка ключа и длина пробы (сколько ячеек осмотрено до неё, считая
    /// её саму). Для отсутствующего ключа — `None`.
    fn find(&self, k: &K) -> Option<(usize, usize)> {
        let mask = self.capacity() - 1;
        let mut i = self.home(k);
        for step in 1..=self.capacity() {
            match &self.slots[i] {
                Slot::Full(ek, _) if ek == k => return Some((i, step)),
                Slot::Empty => return None,
                _ => {}
            }
            i = (i + 1) & mask;
        }
        None
    }

    /// Длина пробы успешного поиска ключа.
    pub fn probe_len(&self, k: &K) -> Option<usize> {
        self.find(k).map(|(_, step)| step)
    }
    // ANCHOR_END: get

    // ANCHOR: remove
    /// Удаление ставит надгробие: пустую ячейку здесь ставить нельзя — она
    /// оборвала бы пробу ключей, что легли за этой ячейкой.
    pub fn remove(&mut self, k: &K) -> Option<V> {
        let (i, _) = self.find(k)?;
        self.live -= 1;
        match std::mem::replace(&mut self.slots[i], Slot::Tomb) {
            Slot::Full(_, v) => Some(v),
            _ => unreachable!(),
        }
    }
    // ANCHOR_END: remove

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.slots.iter().filter_map(|s| match s {
            Slot::Full(k, _) => Some(k),
            _ => None,
        })
    }
}

// ANCHOR: stats
/// Средняя и наибольшая длина пробы успешного поиска по всем живым ключам.
pub fn probe_stats<K: Hash + Eq, V, S: BuildHasher>(t: &OpenTable<K, V, S>) -> (f64, usize) {
    let (mut sum, mut max, mut n) = (0u64, 0, 0u64);
    for k in t.keys() {
        let p = t.probe_len(k).expect("живой ключ находится");
        sum += p as u64;
        max = max.max(p);
        n += 1;
    }
    if n == 0 { (0.0, 0) } else { (sum as f64 / n as f64, max) }
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
// ANCHOR_END: stats
