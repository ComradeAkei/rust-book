//! Раздел 21.2: Release/Acquire и happens-before. Атомарность защищает одну
//! переменную; чтобы через неё безопасно передать ДРУГИЕ данные, нужен
//! порядок. Пара Release-запись / Acquire-чтение одной атомарной переменной
//! создаёт отношение «synchronizes-with»: всё, что писатель сделал ДО
//! Release, читатель, увидевший это значение Acquire-чтением, видит ПОСЛЕ.
//! Это и есть happens-before — каркас всей передачи данных без замка.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;

// ANCHOR: message
/// Передача данных через флаг готовности. Писатель кладёт полезные данные
/// (здесь — обычной Relaxed-записью), затем Release-записью поднимает флаг.
/// Читатель крутится на Acquire-чтении флага и, увидев его поднятым, читает
/// данные — и гарантированно видит их записанными.
///
/// Release у `ready` «запечатывает» всё, что было до неё; Acquire у читателя
/// «распечатывает» это. Поэтому Relaxed для самих данных достаточно: их
/// видимость обеспечивает пара Release/Acquire на `ready`, а не их
/// собственный порядок.
pub fn publish_then_read() -> u64 {
    let data = AtomicU64::new(0);
    let ready = AtomicBool::new(false);

    thread::scope(|s| {
        s.spawn(|| {
            data.store(42, Ordering::Relaxed); // полезные данные
            ready.store(true, Ordering::Release); // публикация: барьер-печать
        });

        // Читатель: ждём публикации, затем читаем данные.
        while !ready.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        // happens-before гарантирует: раз увидели ready == true Acquire-чтением,
        // то видим и data == 42, хотя сама запись была Relaxed.
        data.load(Ordering::Relaxed)
    })
}
// ANCHOR_END: message

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_always_sees_published_data() {
        // Повторяем: happens-before обязан держаться на каждом прогоне.
        for _ in 0..1000 {
            assert_eq!(publish_then_read(), 42);
        }
    }
}
