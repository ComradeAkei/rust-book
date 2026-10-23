//! Задача D: lock-free стек Трайбера с защитой от ABA. Стек на
//! `compare_exchange` головы: push ставит узел и CAS-ом двигает голову на
//! него, pop CAS-ом снимает голову. Беда — ABA: пока поток готовил CAS,
//! голова успела стать другой и снова прежней, CAS «проходит», а стек уже не
//! тот. Лекарство — версионная голова: к индексу пристёгнут счётчик
//! поколений (как тегированный указатель из [главы 17](../ch17/index.md)),
//! и «тот же индекс, но позже» не спутать.
//!
//! Узлы берутся из заранее выделенного пула по индексам, а не из кучи:
//! освобождение узла — возврат в lock-free список свободных, тоже с
//! версионной головой. Так снята и проблема переиспользования памяти.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU64, Ordering};

// ANCHOR: solution
const NIL: u32 = u32::MAX;

/// Голова = (поколение << 32) | индекс. Поколение растёт при каждом успешном
/// CAS, поэтому повтор индекса после цикла push/pop даёт другое значение
/// головы — ABA не проходит.
fn pack(ver: u32, idx: u32) -> u64 {
    ((ver as u64) << 32) | idx as u64
}
fn ver_of(head: u64) -> u32 {
    (head >> 32) as u32
}
fn idx_of(head: u64) -> u32 {
    head as u32
}

struct Node {
    value: UnsafeCell<u64>,
    next: UnsafeCell<u32>,
}

/// Lock-free стек на `N` узлов из пула. `data` — голова стека значений,
/// `free` — голова списка свободных узлов; обе версионные.
pub struct TreiberStack<const N: usize> {
    nodes: [Node; N],
    data: AtomicU64,
    free: AtomicU64,
}

// SAFETY: каждый узел в любой момент принадлежит ровно одному из двух
// стеков; доступ к его полям идёт только у потока, выигравшего CAS за него.
unsafe impl<const N: usize> Sync for TreiberStack<N> {}

impl<const N: usize> TreiberStack<N> {
    pub fn new() -> Self {
        let nodes =
            std::array::from_fn(|_| Node { value: UnsafeCell::new(0), next: UnsafeCell::new(NIL) });
        let stack = TreiberStack {
            nodes,
            data: AtomicU64::new(pack(0, NIL)),
            free: AtomicU64::new(pack(0, NIL)),
        };
        // Сложить все узлы в список свободных (инициализация — без гонки).
        for i in 0..N as u32 {
            let head = stack.free.load(Ordering::Relaxed);
            // SAFETY: инициализация однопоточная.
            unsafe { *stack.nodes[i as usize].next.get() = idx_of(head) };
            stack.free.store(pack(0, i), Ordering::Relaxed);
        }
        stack
    }

    /// Снять узел с версионной головы `head` (lock-free). `None` — пусто.
    fn pop_index(&self, head: &AtomicU64) -> Option<u32> {
        loop {
            let h = head.load(Ordering::Acquire);
            let idx = idx_of(h);
            if idx == NIL {
                return None;
            }
            // SAFETY: читаем next кандидата; если голова изменится, CAS ниже
            // не пройдёт и мы повторим — прочитанное значение не используем.
            let next = unsafe { *self.nodes[idx as usize].next.get() };
            let new = pack(ver_of(h).wrapping_add(1), next);
            if head.compare_exchange_weak(h, new, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                return Some(idx);
            }
        }
    }

    /// Поставить узел `idx` на версионную голову `head` (lock-free).
    fn push_index(&self, head: &AtomicU64, idx: u32) {
        loop {
            let h = head.load(Ordering::Acquire);
            // SAFETY: узел idx принадлежит нам (только что сняли с другого
            // стека), пишем его next до публикации.
            unsafe { *self.nodes[idx as usize].next.get() = idx_of(h) };
            let new = pack(ver_of(h).wrapping_add(1), idx);
            if head.compare_exchange_weak(h, new, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                return;
            }
        }
    }

    /// Положить значение. `Err(v)` — пул узлов исчерпан.
    pub fn push(&self, value: u64) -> Result<(), u64> {
        match self.pop_index(&self.free) {
            Some(idx) => {
                // SAFETY: узел наш (сняли со списка свободных).
                unsafe { *self.nodes[idx as usize].value.get() = value };
                self.push_index(&self.data, idx);
                Ok(())
            }
            None => Err(value),
        }
    }

    /// Снять значение. `None` — стек пуст.
    pub fn pop(&self) -> Option<u64> {
        let idx = self.pop_index(&self.data)?;
        // SAFETY: узел наш (сняли со стека данных).
        let value = unsafe { *self.nodes[idx as usize].value.get() };
        self.push_index(&self.free, idx); // вернуть в пул
        Some(value)
    }
}
// ANCHOR_END: solution

impl<const N: usize> Default for TreiberStack<N> {
    fn default() -> Self {
        Self::new()
    }
}
