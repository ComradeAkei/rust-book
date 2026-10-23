//! Раздел 14.4: `Arc` — счётчик ссылок, который можно делить между
//! потоками.
//!
//! Модуль есть только там, где у процессора есть атомарное
//! чтение-изменение-запись указателя (`target_has_atomic = "ptr"`): на
//! Cortex-M3 и M4 он собирается, на Cortex-M0 — нет, как и `alloc::sync`.

use alloc::sync::Arc;
#[cfg(feature = "std")]
use alloc::vec::Vec;

#[cfg(feature = "std")]
// ANCHOR: spawn
/// Таблица калибровки, общая для потоков опроса. У `thread::spawn`
/// замыкание `'static`: заимствовать таблицу поток не может, поэтому
/// каждый получает свой клон `Arc`, а таблица живёт, пока жив последний.
pub fn calibrate_spawn(table: Arc<[i32; 256]>, blocks: Vec<Vec<u8>>) -> Vec<i64> {
    let handles: Vec<_> = blocks
        .into_iter()
        .map(|raw| {
            let table = Arc::clone(&table);
            std::thread::spawn(move || raw.iter().map(|&b| i64::from(table[usize::from(b)])).sum())
        })
        .collect();
    handles.into_iter().map(|h| h.join().unwrap()).collect()
}
// ANCHOR_END: spawn

#[cfg(feature = "std")]
// ANCHOR: scope
/// То же без `Arc`: потоки `thread::scope` гарантированно закончатся до
/// выхода из функции, и им хватает обычной ссылки на таблицу — ни одной
/// атомарной операции со счётчиком.
pub fn calibrate_scoped(table: &[i32; 256], blocks: &[Vec<u8>]) -> Vec<i64> {
    std::thread::scope(|s| {
        let handles: Vec<_> = blocks
            .iter()
            .map(|raw| s.spawn(move || raw.iter().map(|&b| i64::from(table[usize::from(b)])).sum()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}
// ANCHOR_END: scope

/// `Arc` отправляется в поток, только если значение `Send + Sync`: клоны
/// в разных потоках читают его одновременно. `Cell` — не `Sync`:
///
/// ```compile_fail,E0277
/// use std::cell::Cell;
/// use std::sync::Arc;
/// let hits = Arc::new(Cell::new(0u32));
/// let mine = Arc::clone(&hits);
/// std::thread::spawn(move || mine.set(1));
/// ```
// ANCHOR: make_mut
/// Как и у `Rc`: копия, только если таблицу держит кто-то ещё.
pub fn patch(table: &mut Arc<[i32; 256]>, index: u8, value: i32) {
    Arc::make_mut(table)[usize::from(index)] = value;
}
// ANCHOR_END: make_mut

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use alloc::vec;

    fn table() -> [i32; 256] {
        core::array::from_fn(|i| i as i32 * 3 - 100)
    }

    #[test]
    fn spawn_and_scope_agree() {
        let blocks = vec![vec![0u8, 1, 2], vec![255; 10], vec![]];
        let shared = Arc::new(table());
        let a = calibrate_spawn(Arc::clone(&shared), blocks.clone());
        let b = calibrate_scoped(&shared, &blocks);
        assert_eq!(a, b);
        assert_eq!(a, [-300 + 9, 10 * (255 * 3 - 100), 0]);
        assert_eq!(Arc::strong_count(&shared), 1, "клоны потоков уничтожены");
    }

    #[test]
    fn patch_copies_shared_table() {
        let mut t = Arc::new(table());
        let old = Arc::clone(&t);
        patch(&mut t, 0, 42);
        assert_eq!((old[0], t[0]), (-100, 42));
        let p = Arc::as_ptr(&t);
        patch(&mut t, 1, 43);
        assert_eq!(Arc::as_ptr(&t), p);
    }
}
