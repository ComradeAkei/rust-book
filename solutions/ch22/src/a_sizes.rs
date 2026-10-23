//! Задача A: размеры future шести `async fn`.
//!
//! Сначала предскажите ответы без компилятора, затем проверьте: `measured`
//! меряет настоящие размеры на x86-64. Размер автомата — самое толстое
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

// ANCHOR: predicted
/// Предсказания с выкладками (x86-64, rustc 1.97). Каждый автомат хранит
/// дискриминант состояния (1 байт: «не начат», «готов», «паника» и по
/// одному на каждый `.await`), аргументы `async fn` и всё, что живо через
/// `.await`; состояния перекрываются, как варианты `enum`.
pub fn predicted() -> [usize; 6] {
    [
        // s0: ни одного `.await`, аргументов нет — один дискриминант.
        1,
        // s1: аргумент x (8) хранится весь срок жизни автомата, а в
        // состоянии `.await` живёт ещё его копия (8) и `Pause` (1); с
        // дискриминантом 18 → кратно выравниванию 8 → 24.
        24,
        // s2: буфер (64) живёт через `.await`, плюс `Pause` (1) и
        // дискриминант (1); выравнивание 1 — 66.
        66,
        // s3: буфер умер внутри блока до `.await`; живы n (8), `Pause` (1),
        // дискриминант (1): 10 → 16.
        16,
        // s4: пока ждём второй s2, жив результат первого (8) и future
        // второго s2 (66). Дискриминант лежит в начале, usize выравнивается
        // на смещение 8, future s2 занимает 16..82; 82 → 88. Плотнее (80)
        // вышло бы, переставь компилятор поля, — автоматы он так не
        // переставляет.
        88,
        // s5: срез v (16) хранится как аргумент и ещё раз — через `.await`,
        // плюс first (1), `Pause` (1), дискриминант (1): 35 → 40.
        40,
    ]
}
// ANCHOR_END: predicted
