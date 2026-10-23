//! Раздел 20.1: запуск потоков. `thread::spawn` берёт замыкание и исполняет
//! его на новом потоке операционной системы; `JoinHandle::join` дожидается
//! результата. Чтобы поток владел тем, что использует, замыкание помечают
//! `move` — данные переезжают в поток, а не одалживаются у кадра, который
//! может исчезнуть раньше.

use std::thread;

// ANCHOR: spawn
/// Посчитать сумму на отдельном потоке и вернуть её в вызывающий. Вектор
/// отдаётся потоку во владение (`move`): после `spawn` исходная переменная
/// недоступна — компилятор это гарантирует, поэтому гонки за `data` нет.
pub fn sum_on_thread(data: Vec<u64>) -> u64 {
    let handle = thread::spawn(move || data.iter().sum::<u64>());
    // join возвращает Result: Err, если поток паниковал. Здесь мы знаем, что
    // тело не паникует, поэтому разворачиваем.
    handle.join().unwrap()
}
// ANCHOR_END: spawn

// ANCHOR: many
/// Запустить `n` потоков, каждый возводит свой номер в квадрат, собрать
/// результаты по порядку. `JoinHandle<T>` несёт тип результата потока,
/// поэтому `join` отдаёт именно `T` (здесь `u64`).
pub fn squares(n: u64) -> Vec<u64> {
    let handles: Vec<_> = (0..n).map(|i| thread::spawn(move || i * i)).collect();
    // Собираем в порядке запуска: join по очереди, а не по готовности.
    handles.into_iter().map(|h| h.join().unwrap()).collect()
}
// ANCHOR_END: many

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_runs_on_another_thread() {
        assert_eq!(sum_on_thread(vec![1, 2, 3, 4]), 10);
    }

    #[test]
    fn squares_keep_order() {
        assert_eq!(squares(5), vec![0, 1, 4, 9, 16]);
    }
}
