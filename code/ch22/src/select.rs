//! Раздел 22.5: `select`, `join` и отмена через `drop`.
//!
//! Отменить future — значит просто перестать её опрашивать и уронить.
//! Никакого исключения внутрь не летит: её автомат уничтожается в той
//! точке `.await`, где стоял, со всеми локальными переменными этого
//! состояния. `select` ждёт первую из двух future и роняет вторую; `join`
//! ждёт обе. Опасность отмены — в данных, которые future успела забрать из
//! внешнего мира, но не успела отдать: они уходят вместе с её состоянием.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

// ANCHOR: select
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

/// Ждать первую готовую из двух future; проигравшая будет уронена вместе
/// с `Select`. `Unpin` требуем для простоты: закрепите future через
/// `pin!` и передайте `Pin<&mut F>` — он `Unpin` всегда.
pub fn select<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    Select { a, b }
}

pub struct Select<A, B> {
    a: A,
    b: B,
}

impl<A: Future + Unpin, B: Future + Unpin> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(v) = Pin::new(&mut self.a).poll(cx) {
            return Poll::Ready(Either::Left(v));
        }
        if let Poll::Ready(v) = Pin::new(&mut self.b).poll(cx) {
            return Poll::Ready(Either::Right(v));
        }
        Poll::Pending // обе опрошены с одним cx: разбудит любая
    }
}
// ANCHOR_END: select

// ANCHOR: join
/// Ждать обе future и вернуть оба результата. Готовая раньше хранит свой
/// результат, пока ждём вторую, — и больше не опрашивается.
pub fn join<A: Future + Unpin, B: Future + Unpin>(a: A, b: B) -> Join<A, B> {
    Join { a: MaybeDone::Pending(a), b: MaybeDone::Pending(b) }
}

pub struct Join<A: Future, B: Future> {
    a: MaybeDone<A>,
    b: MaybeDone<B>,
}

enum MaybeDone<F: Future> {
    Pending(F),
    Done(F::Output),
    Taken,
}

impl<F: Future + Unpin> MaybeDone<F> {
    fn poll(&mut self, cx: &mut Context<'_>) -> bool {
        if let MaybeDone::Pending(f) = self {
            match Pin::new(f).poll(cx) {
                Poll::Ready(v) => *self = MaybeDone::Done(v),
                Poll::Pending => return false,
            }
        }
        true
    }
    fn take(&mut self) -> F::Output {
        match core::mem::replace(self, MaybeDone::Taken) {
            MaybeDone::Done(v) => v,
            _ => unreachable!("результат забирают один раз, после готовности"),
        }
    }
}

// Поля опрашиваем только через &mut, ничего не закрепляя: A и B — Unpin.
impl<A: Future + Unpin, B: Future + Unpin> Unpin for Join<A, B> {}

impl<A: Future + Unpin, B: Future + Unpin> Future for Join<A, B> {
    type Output = (A::Output, B::Output);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let a_done = self.a.poll(cx);
        let b_done = self.b.poll(cx);
        if a_done && b_done { Poll::Ready((self.a.take(), self.b.take())) } else { Poll::Pending }
    }
}
// ANCHOR_END: join

// ANCHOR: lossy
/// Источник байтов: отдаёт по одному за опрос и `Pending` в промежутках —
/// как сокет, в который данные приходят кусками.
pub struct Trickle<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    ready: bool,
}

impl<'a> Trickle<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Trickle { data, pos: 0, ready: false }
    }

    /// Следующий байт: через опрос, как настоящий ввод.
    pub fn next_byte(&mut self) -> NextByte<'_, 'a> {
        NextByte { src: self }
    }
}

pub struct NextByte<'s, 'a> {
    src: &'s mut Trickle<'a>,
}

impl Future for NextByte<'_, '_> {
    type Output = Option<u8>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<u8>> {
        let src = &mut *self.src;
        if !src.ready {
            src.ready = true; // байт «ещё в пути»
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        src.ready = false;
        let b = src.data.get(src.pos).copied();
        src.pos += b.is_some() as usize;
        Poll::Ready(b)
    }
}

/// Прочитать `N` байт в локальный буфер; вернуть буфер и сколько прочитано.
/// Байт, однажды забранный из источника, живёт только в `out` — в состоянии
/// автомата. Отмена между байтами роняет `out`, и забранное пропадает:
/// источник его уже не вернёт.
pub async fn read_exact<const N: usize>(src: &mut Trickle<'_>) -> ([u8; N], usize) {
    let mut out = [0u8; N];
    let mut len = 0;
    while len < N {
        match src.next_byte().await {
            Some(b) => {
                out[len] = b;
                len += 1;
            }
            None => break,
        }
    }
    (out, len)
}
// ANCHOR_END: lossy

#[cfg(test)]
mod tests {
    use super::*;
    use crate::future::{CountDown, poll_to_end};
    use core::pin::pin;

    #[test]
    fn select_returns_first_and_drops_loser() {
        struct Flag<'a>(&'a core::cell::Cell<bool>, CountDown);
        impl Drop for Flag<'_> {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        impl Future for Flag<'_> {
            type Output = u32;
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
                Pin::new(&mut self.1).poll(cx)
            }
        }
        let dropped = core::cell::Cell::new(false);
        let slow = Flag(&dropped, CountDown::new(10));
        let (res, _) = poll_to_end(select(CountDown::new(1), slow));
        assert!(matches!(res, Either::Left(2)));
        assert!(dropped.get(), "проигравшая future уронена — это и есть отмена");
    }

    #[test]
    fn join_waits_for_both() {
        let ((a, b), polls) = poll_to_end(join(CountDown::new(1), CountDown::new(3)));
        assert_eq!((a, b, polls), (2, 4, 4));
    }

    // ANCHOR: lossy_test
    #[test]
    fn cancelled_read_loses_consumed_bytes() {
        let mut src = Trickle::new(b"HEADpayload");
        {
            // Читаем 4 байта, но «тайм-аут» срабатывает на третьем опросе.
            let read = pin!(read_exact::<4>(&mut src));
            let timeout = CountDown::new(2);
            let (res, _) = poll_to_end(select(read, timeout));
            assert!(matches!(res, Either::Right(_)), "победил тайм-аут");
        } // здесь уронен автомат read_exact вместе с буфером `out`
        assert_eq!(src.pos, 2, "два байта уже забраны из источника…");
        let ((rest, n), _) = poll_to_end(read_exact::<4>(&mut src));
        assert_eq!(&rest[..n], b"ADpa", "…и пропали: следующий read начат с середины");
    }
    // ANCHOR_END: lossy_test
}
