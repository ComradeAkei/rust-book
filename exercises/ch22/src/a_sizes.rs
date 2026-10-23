//! Задача A: размеры future шести `async fn`.
//!
//! Сначала предскажите ответы без компилятора и впишите их в `predicted`,
//! затем проверьте: `measured` меряет настоящие размеры на x86-64. Размер автомата — самое толстое
//! состояние (всё, что живо через его `.await`) плюс дискриминант, с
//! выравниванием. Раскладка автоматов не стабилизирована: ответ верен для
//! rustc 1.97 и может измениться в другой версии.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

// ANCHOR: futures
/// Точка приостановки размером 1 байт: один `Pending`, затем `Ready`.
pub struct Pause(bool);

impl Future for Pause {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub fn pause() -> Pause {
    Pause(false)
}

pub async fn s0() -> u64 {
    42
}

pub async fn s1(x: u64) -> u64 {
    pause().await;
    x
}

pub async fn s2() -> usize {
    let buf = [7u8; 64];
    pause().await;
    buf.iter().map(|&b| b as usize).sum()
}

pub async fn s3() -> usize {
    let n = {
        let buf = [7u8; 64];
        buf.iter().map(|&b| b as usize).sum::<usize>()
    };
    pause().await;
    n
}

pub async fn s4() -> usize {
    s2().await + s2().await
}

pub async fn s5(v: &[u8]) -> usize {
    let first = v[0];
    pause().await;
    v.len() + first as usize
}

/// Настоящие размеры future `s0`–`s5` в байтах.
pub fn measured() -> [usize; 6] {
    let v = [1u8, 2, 3];
    [
        size_of_val(&s0()),
        size_of_val(&s1(1)),
        size_of_val(&s2()),
        size_of_val(&s3()),
        size_of_val(&s4()),
        size_of_val(&s5(&v)),
    ]
}
// ANCHOR_END: futures

// ANCHOR: skeleton
/// Ваши предсказания размеров future `s0`–`s5` в байтах (x86-64). Для
/// каждого числа запишите выкладку: что живо через `.await`, где
/// дискриминант, как выравнивание округляет итог.
pub fn predicted() -> [usize; 6] {
    todo!("раздел 22.7, задача A")
}
// ANCHOR_END: skeleton
