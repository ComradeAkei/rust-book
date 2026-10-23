//! Раздел 12.6: что замыкание наследует от захваченного — `Copy`, `Clone`,
//! `Send`, `Sync`, — и асинхронные замыкания.

use core::pin::pin;
use core::task::{Context, Poll, Waker};

// ANCHOR: copy
fn copy<T: Copy>(t: &T) -> T {
    *t
}

/// Замыкание `Copy`, если `Copy` всё захваченное. Общая ссылка — `Copy`,
/// так что замыкание, которое только читает, копируется всегда.
pub fn copies() -> (u32, u32) {
    let k = 3u32;
    let r = &k;
    let by_value = move |x: u32| x * k;
    let by_ref = |x: u32| x * *r;
    let (a, b) = (copy(&by_value), copy(&by_ref));
    (a(2) + by_value(1), b(2) + by_ref(1))
}
// ANCHOR_END: copy

/// Изменяемая ссылка не `Copy`, и замыкание, которое её держит, тоже.
///
/// ```compile_fail,E0277
/// fn copy<T: Copy>(t: &T) -> T { *t }
/// let mut n = 0u32;
/// let mut bump = || n += 1;
/// let mut other = copy(&bump);
/// ```
pub const fn not_copy() {}

// ANCHOR: send
pub fn is_send<T: Send>(_: &T) {}
pub fn is_sync<T: Sync>(_: &T) {}

/// `Send` и `Sync` выводятся из захваченного по тем же правилам, что для
/// структуры с такими полями. `Cell` можно передать в другой поток, но
/// нельзя делить между потоками: замыкание с `Cell` по значению — `Send`,
/// а с `&Cell` — нет.
pub fn send_sync() -> u32 {
    let cell = core::cell::Cell::new(5u32);
    let borrowed = || cell.get();
    let n = borrowed();
    let owned = move || cell.get();
    is_send(&owned);
    n + owned()
}
// ANCHOR_END: send

/// Захват `&Cell` требует `Cell: Sync`, а его нет.
///
/// ```compile_fail,E0277
/// fn is_send<T: Send>(_: &T) {}
/// let cell = core::cell::Cell::new(5u32);
/// let borrowed = || cell.get();
/// is_send(&borrowed);
/// ```
///
/// `Rc` не `Send` вовсе: его счётчик меняется без атомарных операций.
///
/// ```compile_fail,E0277
/// let rc = std::rc::Rc::new(5u32);
/// let h = std::thread::spawn(move || *rc + 1);
/// ```
pub const fn not_send() {}

// ANCHOR: leak
/// `impl Trait` в результате скрывает тип, но не автотрейты: `Send` и
/// `Sync` скрытого типа видны снаружи, хотя в сигнатуре их нет. Счётчик
/// из раздела 12.4 держит `u32` и потому `Send`.
pub fn leaks_send() -> u32 {
    let mut c = crate::ret::make_counter(1);
    is_send(&c);
    c()
}
// ANCHOR_END: leak

/// А `Copy` и `Clone` — обычные трейты, и через `impl Fn` они не видны,
/// даже если само замыкание `Copy`.
///
/// ```compile_fail,E0277
/// fn copy<T: Copy>(t: &T) -> T { *t }
/// let p = ch12::ret::pipeline(100, 3);
/// let q = copy(&p);
/// ```
pub const fn no_copy_leak() {}

// ANCHOR: block-on
/// Простейший исполнитель: опрашивать, пока не будет готово. Годится для
/// future, которые не ждут ничего внешнего, как в этом разделе; настоящие
/// исполнители — в главе 22.
pub fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}
// ANCHOR_END: block-on

// ANCHOR: async-closure
/// На каждый байт — вызов и ожидание. Future, который возвращает `f(b)`,
/// заимствует само замыкание, и его состояние доступно внутри.
pub async fn each<F: AsyncFnMut(u8)>(data: &[u8], mut f: F) {
    for &b in data {
        f(b).await;
    }
}

pub fn async_sum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    block_on(each(data, async |b| sum += u32::from(b)));
    sum
}
// ANCHOR_END: async-closure

// ANCHOR: async-move
/// `move` у асинхронного замыкания переносит `sum` в структуру замыкания:
/// каждый future прибавляет к этой копии, а переменная функции остаётся
/// нулём. В отличие от `async move`-блока ниже, здесь rustc 1.97 молчит.
pub fn async_move_sum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    block_on(each(data, async move |b| sum += u32::from(b)));
    sum
}
// ANCHOR_END: async-move

// ANCHOR: async-old
/// Старая запись: замыкание, которое возвращает future. Future не может
/// заимствовать у замыкания, и остаётся `async move`, который копирует
/// `sum` в каждый future. Ловушка из раздела 12.1 в квадрате: rustc
/// предупреждает, но сумма остаётся нулём.
pub async fn each_old<F, Fut>(data: &[u8], mut f: F)
where
    F: FnMut(u8) -> Fut,
    Fut: Future<Output = ()>,
{
    for &b in data {
        f(b).await;
    }
}

#[allow(unused_assignments)]
pub fn async_old_sum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    block_on(each_old(data, |b| async move { sum += u32::from(b) }));
    sum
}
// ANCHOR_END: async-old

/// Без `move` future держал бы ссылки на параметр `b` (E0373: future
/// переживает вызов) и на `sum`, полученную замыканием, а пережить вызов
/// `FnMut` такая ссылка не может («captured variable cannot escape `FnMut`
/// closure body», без кода ошибки).
///
/// ```compile_fail,E0373
/// use ch12::auto::{block_on, each_old};
/// let mut sum = 0u32;
/// block_on(each_old(&[1, 2, 3], |b| async { sum += u32::from(b) }));
/// ```
pub const fn async_borrow() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_traits() {
        assert_eq!(copies(), (9, 9));
        assert_eq!(send_sync(), 10);
        assert_eq!(leaks_send(), 2);
    }

    #[test]
    fn async_closures() {
        assert_eq!(async_sum(&[1, 2, 3]), 6);
        assert_eq!(async_old_sum(&[1, 2, 3]), 0);
        assert_eq!(async_move_sum(&[1, 2, 3]), 0);
    }
}
