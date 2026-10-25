//! Задача D: lock-free стек Трайбера с защитой от ABA. Условие — раздел 21.7.
//!
//! Стек на `compare_exchange` головы: push ставит узел и CAS-ом двигает голову
//! на него, pop CAS-ом снимает голову. Беда — ABA: пока поток готовил CAS,
//! голова успела стать другой и снова прежней, CAS «проходит», а стек уже не
//! тот. Лекарство — версионная голова: к индексу пристёгнут счётчик поколений
//! (как тегированный указатель из [главы 17](../ch17/index.md)), и «тот же
//! индекс, но позже» не спутать. Узлы берутся из заранее выделенного пула по
//! индексам, а не из кучи; освобождение — возврат в lock-free список свободных
//! с такой же версионной головой.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// ANCHOR: skeleton
const NIL: u32 = u32::MAX;

/// Голова = (поколение << 32) | индекс. Поколение растёт при каждом успешном
/// CAS, поэтому повтор индекса после цикла push/pop даёт другое значение
/// головы — ABA не проходит.
fn pack(ver: u32, idx: u32) -> u64 {
    ((ver as u64) << 32) | idx as u64
}
#[allow(dead_code)]
fn ver_of(head: u64) -> u32 {
    (head >> 32) as u32
}
fn idx_of(head: u64) -> u32 {
    head as u32
}

#[allow(dead_code)]
struct Node {
    value: UnsafeCell<u64>,
    // Атомарный: его читает и отставший pop_index, проигрывающий CAS.
    next: AtomicU32,
}

/// Lock-free стек на `N` узлов из пула. `data` — голова стека значений,
/// `free` — голова списка свободных узлов; обе версионные.
#[allow(dead_code)]
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
    /// Цикл: прочитайте `head.load(Acquire)`, если индекс `NIL` — верните
    /// `None`; иначе прочитайте `next` кандидата (`load(Relaxed)`: узел может
    /// уже принадлежать другому потоку — тогда CAS не пройдёт, а атомарность
    /// спасает от гонки данных), соберите новую голову `pack(ver_of(h) + 1,
    /// next)` и попробуйте `compare_exchange_weak` с `AcqRel` при успехе.
    /// Поколение растёт на каждый успех — здесь и живёт защита от ABA.
    #[allow(dead_code)]
    fn pop_index(&self, head: &AtomicU64) -> Option<u32> {
        let _ = head;
        todo!("раздел 21.7, задача D")
    }

    /// Поставить узел `idx` на версионную голову `head` (lock-free). Цикл:
    /// прочитайте голову, запишите в `next` узла текущий индекс головы
    /// (`store(Relaxed)`: опубликует его Release-часть CAS), соберите
    /// `pack(ver_of(h) + 1, idx)` и повторяйте `compare_exchange_weak`
    /// (`AcqRel`), пока не пройдёт.
    #[allow(dead_code)]
    fn push_index(&self, head: &AtomicU64, idx: u32) {
        let _ = (head, idx);
        todo!("раздел 21.7, задача D")
    }

    /// Положить значение. `Err(v)` — пул узлов исчерпан. Снимите узел со списка
    /// свободных (`pop_index(&self.free)`); если пусто — `Err(value)`. Иначе
    /// запишите значение в узел и поставьте его на стек данных
    /// (`push_index(&self.data, idx)`).
    pub fn push(&self, value: u64) -> Result<(), u64> {
        let _ = value;
        todo!("раздел 21.7, задача D")
    }

    /// Снять значение. `None` — стек пуст. Снимите узел со стека данных
    /// (`pop_index(&self.data)`), прочитайте его значение и верните узел в пул
    /// (`push_index(&self.free, idx)`).
    pub fn pop(&self) -> Option<u64> {
        todo!("раздел 21.7, задача D")
    }
}
// ANCHOR_END: skeleton

impl<const N: usize> Default for TreiberStack<N> {
    fn default() -> Self {
        Self::new()
    }
}
