//! Раздел 3.3: флаг drop в машинном коде.
//!
//! Модуль без `std`: ресурс с деструктором — аренда канала DMA, деструктор
//! возвращает канал вызовом `ch03_release`. Все функции
//! `#[inline(never)]`, чтобы LLVM не растворил флаг в окружающем коде.

use core::hint::black_box;

// ANCHOR: lease
/// Аренда канала DMA. Пока значение живо, канал занят.
pub struct Lease(pub u8);

impl Drop for Lease {
    fn drop(&mut self) {
        ch03_release(self.0);
    }
}

/// Вернуть канал контроллеру DMA.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_release(channel: u8) {
    black_box(channel);
}

/// Запустить передачу. Аренда переходит к передаче и закончится вместе с
/// ней.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_start(lease: Lease) {
    black_box(&lease);
}

/// Готово ли устройство к передаче на попытке `i`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_ready(i: u32) -> bool {
    black_box(i) % 7 == 3
}

/// Работа после поиска, которую компилятор не может предсказать.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_work() {
    black_box(());
}
// ANCHOR_END: lease

// ANCHOR: flag
/// Канал уходит передаче, только если устройство ответило за `tries`
/// попыток. Вернуть ли его в конце функции, известно лишь во время
/// выполнения.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_send_when_ready(lease: Lease, tries: u32) -> bool {
    let mut sent = false;
    for i in 0..tries {
        if ch03_ready(i) {
            ch03_start(lease);
            sent = true;
            break;
        }
    }
    for _ in 0..tries {
        ch03_work();
    }
    sent
} // здесь `drop(lease)`, если аренда не перемещена
// ANCHOR_END: flag

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_decides_at_runtime() {
        // ch03_ready(3) == true: аренда перемещена в ch03_start.
        assert!(ch03_send_when_ready(Lease(1), 10));
        assert!(!ch03_send_when_ready(Lease(1), 3));
        // Флаг живёт в кадре функции, а не в значении.
        assert_eq!(size_of::<Lease>(), 1);
    }
}
