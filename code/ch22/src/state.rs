//! Раздел 22.2: `async fn` как конечный автомат.
//!
//! Компилятор превращает `async fn` в анонимный тип-автомат: перечисление
//! состояний «ещё не начата», «стоит на первом `.await`», «на втором»…,
//! «завершена». В каждом состоянии хранится ровно то, что живо через эту
//! точку приостановки. Поэтому размер future — размер самого «толстого»
//! состояния плюс дискриминант, и его видно через `size_of_val`.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::future::{CountDown, yield_now};

// ANCHOR: async_fn
/// Обычная `async fn`: `x` вычислена до `.await` и нужна после, значит,
/// автомат обязан хранить её, пока стоит на этой точке.
pub async fn add_after(n: u32, wait: CountDown) -> u32 {
    let x = n * 2;
    let polls = wait.await;
    x + polls
}
// ANCHOR_END: async_fn

// ANCHOR: desugared
/// Та же функция, написанная вручную, — примерно то, во что её превращает
/// компилятор. Три состояния: не начата, ждёт `CountDown`, завершена.
pub enum AddAfter {
    Start { n: u32, wait: CountDown },
    Waiting { x: u32, wait: CountDown },
    Done,
}

pub fn add_after_by_hand(n: u32, wait: CountDown) -> AddAfter {
    AddAfter::Start { n, wait } // вызов ничего не исполняет: только упаковка
}

impl Future for AddAfter {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        loop {
            match &mut *self {
                AddAfter::Start { n, wait } => {
                    // Код до первого .await исполняется при первом опросе.
                    let x = *n * 2;
                    let wait = core::mem::replace(wait, CountDown::new(0));
                    *self = AddAfter::Waiting { x, wait };
                }
                AddAfter::Waiting { x, wait } => {
                    let polls = match Pin::new(wait).poll(cx) {
                        Poll::Ready(p) => p,
                        Poll::Pending => return Poll::Pending, // стоим здесь
                    };
                    let out = *x + polls;
                    *self = AddAfter::Done;
                    return Poll::Ready(out);
                }
                AddAfter::Done => panic!("future опрошена после Ready"),
            }
        }
    }
}
// ANCHOR_END: desugared

// ANCHOR: sizes
/// Буфер нужен после `.await` — автомат хранит все 1024 байта.
pub async fn buffer_across() -> usize {
    let buf = [1u8; 1024];
    yield_now().await;
    buf.iter().map(|&b| b as usize).sum()
}

/// Буфер умирает в блоке до `.await` — в автомат он не попадает.
pub async fn buffer_scoped() -> usize {
    let n = {
        let buf = [1u8; 1024];
        buf.iter().map(|&b| b as usize).sum::<usize>()
    };
    yield_now().await;
    n
}

/// `drop` до `.await` автомат НЕ уменьшает: место под переменную отводится
/// на всю её область видимости, а не до последнего использования. (clippy
/// здесь прав: такой `drop` бесполезен — ради этого пример и написан.)
#[allow(clippy::drop_non_drop)]
pub async fn buffer_dropped() -> usize {
    let buf = Buf1k([1u8; 1024]); // не Copy: drop перемещает, а не копирует
    let n = buf.sum();
    drop(buf);
    yield_now().await;
    n
}

/// Два буфера в разных состояниях делят одни и те же байты, как варианты
/// перечисления: автомат меньше суммы буферов.
pub async fn two_states() -> usize {
    let a = {
        let a = [1u8; 100];
        yield_now().await;
        a.len()
    };
    let b = {
        let b = [2u8; 200];
        yield_now().await;
        b.len()
    };
    a + b
}

/// Вложенная future хранится внутри внешней целиком.
pub async fn nested() -> usize {
    buffer_across().await + 1
}

/// Аргумент `async fn` лежит в автомате всю его жизнь, даже если нужен
/// только до первого `.await`.
pub async fn with_arg(x: u64) -> u64 {
    let z = x + 1;
    yield_now().await;
    z
}
// ANCHOR_END: sizes

/// Не-`Copy` буфер для `buffer_dropped`: `drop` перемещает его, а не
/// копирует, — и всё равно автомат хранит все 1024 байта.
pub struct Buf1k(pub [u8; 1024]);

impl Buf1k {
    pub fn sum(&self) -> usize {
        self.0.iter().map(|&b| b as usize).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::future::poll_to_end;

    #[test]
    fn hand_written_automaton_matches_async_fn() {
        let (a, pa) = poll_to_end(add_after(5, CountDown::new(3)));
        let (b, pb) = poll_to_end(add_after_by_hand(5, CountDown::new(3)));
        assert_eq!((a, pa), (b, pb));
        assert_eq!(a, 10 + 4);
    }

    #[test]
    fn all_size_examples_run() {
        assert_eq!(poll_to_end(buffer_across()).0, 1024);
        assert_eq!(poll_to_end(buffer_scoped()).0, 1024);
        assert_eq!(poll_to_end(buffer_dropped()).0, 1024);
        assert_eq!(poll_to_end(two_states()).0, 300);
        assert_eq!(poll_to_end(nested()).0, 1025);
        assert_eq!(poll_to_end(with_arg(41)).0, 42);
    }

    /// Гарантия языка: то, что нужно после `.await`, хранится в автомате.
    #[test]
    fn guaranteed_live_buffer_is_stored() {
        assert!(size_of_val(&buffer_across()) >= 1024);
        assert!(size_of_val(&nested()) >= size_of_val(&buffer_across()));
    }

    /// Наблюдения rustc 1.97 на x86-64: раскладка автоматов не
    /// стабилизирована и может измениться в другой версии компилятора.
    mod observed {
        use super::*;

        #[test]
        fn observed_future_sizes() {
            assert_eq!(size_of_val(&buffer_across()), 1026);
            assert_eq!(size_of_val(&buffer_scoped()), 16);
            assert_eq!(size_of_val(&buffer_dropped()), 1040);
            assert_eq!(size_of_val(&two_states()), 224);
            assert_eq!(size_of_val(&nested()), 1027);
            assert_eq!(size_of_val(&with_arg(1)), 24);
        }
    }
}
