//! Раздел 20.5: передача сообщений и ожидание. Вместо общей памяти под
//! замком потоки могут общаться сообщениями: канал `mpsc` (multiple
//! producer, single consumer) переносит значения от многих отправителей к
//! одному получателю, передавая владение. `Condvar` же позволяет потоку
//! уснуть до условия и быть разбуженным — без активного ожидания в цикле.

use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// ANCHOR: mpsc
/// `producers` потоков шлют числа в один канал; получатель суммирует всё,
/// что пришло. `Sender` клонируется по одному на отправителя; когда последний
/// `Sender` уничтожен, итератор по `Receiver` завершается — отсюда сумма без
/// явного «сколько всего сообщений».
pub fn channel_sum(producers: u64, per_producer: u64) -> u64 {
    let (tx, rx) = mpsc::channel::<u64>();
    for p in 0..producers {
        let tx = tx.clone();
        thread::spawn(move || {
            for i in 0..per_producer {
                // Отправка переносит значение в канал.
                tx.send(p * per_producer + i).unwrap();
            }
        });
    }
    // Исходный tx надо уронить, иначе rx будет ждать его вечно.
    drop(tx);
    rx.iter().sum()
}
// ANCHOR_END: mpsc

// ANCHOR: condvar
/// Ожидание условия без активного цикла. Пара `Mutex<bool>` + `Condvar`:
/// рабочий поток ставит флаг и будит ожидающих, главный засыпает на
/// `wait` в цикле, пока флаг не станет `true`. Процессор на ожидании не
/// крутится — поток снят с выполнения до `notify`.
pub fn wait_for_ready() -> &'static str {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let worker = Arc::clone(&pair);
    thread::spawn(move || {
        let (lock, cvar) = &*worker;
        *lock.lock().unwrap() = true;
        // Разбудить всех, кто ждёт на этой переменной условия.
        cvar.notify_all();
    });

    let (lock, cvar) = &*pair;
    let mut ready = lock.lock().unwrap();
    // wait освобождает замок на время сна и снова берёт при пробуждении;
    // цикл while защищает от ложных пробуждений (то же делает wait_while).
    while !*ready {
        ready = cvar.wait(ready).unwrap();
    }
    "готово"
}
// ANCHOR_END: condvar

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_collects_from_all_producers() {
        // Сумма чисел 0..(4*250) = 0..1000.
        let expected: u64 = (0..1000).sum();
        assert_eq!(channel_sum(4, 250), expected);
    }

    #[test]
    fn condvar_wakes_the_waiter() {
        assert_eq!(wait_for_ready(), "готово");
    }
}
