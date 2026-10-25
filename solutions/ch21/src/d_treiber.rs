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
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

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
    // Атомарный: его читает и отставший pop_index, проигрывающий CAS.
    next: AtomicU32,
}

/// Lock-free стек на `N` узлов из пула. `data` — голова стека значений,
/// `free` — голова списка свободных узлов; обе версионные.
pub struct TreiberStack<const N: usize> {
    nodes: [Node; N],
    data: AtomicU64,
    free: AtomicU64,
}

// SAFETY: каждый узел в любой момент принадлежит ровно одному из двух
// стеков, и `value` узла читает и пишет только поток, выигравший CAS за
// него. `next` атомарный: его читает и поток, чей CAS потом не пройдёт, пока
// новый владелец узла пишет `next` в push_index, — обычная ячейка дала бы
// здесь гонку данных.
unsafe impl<const N: usize> Sync for TreiberStack<N> {}

impl<const N: usize> TreiberStack<N> {
    pub fn new() -> Self {
        let nodes =
            std::array::from_fn(|_| Node { value: UnsafeCell::new(0), next: AtomicU32::new(NIL) });
        let stack = TreiberStack {
            nodes,
            data: AtomicU64::new(pack(0, NIL)),
            free: AtomicU64::new(pack(0, NIL)),
        };
        // Сложить все узлы в список свободных (инициализация — без гонки).
        for i in 0..N as u32 {
            let head = stack.free.load(Ordering::Relaxed);
            // Инициализация однопоточная — хватает Relaxed.
            stack.nodes[i as usize].next.store(idx_of(head), Ordering::Relaxed);
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
            // Читаем next кандидата. Узел мог уже уйти к другому потоку, и
            // тот пишет его next в push_index: тогда голова изменилась, CAS
            // ниже не пройдёт, и прочитанное выбросится. Но само чтение
            // одновременно с записью было бы гонкой данных (UB), будь next
            // обычной ячейкой, — поэтому он атомарный. Relaxed хватает:
            // next, записанный до публикации узла, виден через
            // Acquire-чтение головы.
            let next = self.nodes[idx as usize].next.load(Ordering::Relaxed);
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
            // Узел idx наш (только что сняли с другого стека); next пишем до
            // публикации CAS-ом с Release. Атомарно: его может в этот момент
            // читать отставший pop_index.
            self.nodes[idx as usize].next.store(idx_of(h), Ordering::Relaxed);
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
