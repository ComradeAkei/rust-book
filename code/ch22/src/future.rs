//! Раздел 22.1: `Future`, `poll`, `Context` и `Waker`.
//!
//! Future — значение, которое можно *опрашивать*: `poll` либо отдаёт
//! `Poll::Ready(результат)`, либо `Poll::Pending` — «ещё не готово». Сама
//! future ничего не исполняет в фоне: работа идёт только внутри `poll`.
//! Вернув `Pending`, future обязана позаботиться, чтобы её опросили снова, —
//! для этого `Context` несёт `Waker`, ручку «разбуди задачу».

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

// ANCHOR: countdown
/// Future, готовая на `n + 1`-м опросе. Каждый раз, возвращая `Pending`,
/// она сама будит свою задачу (`wake_by_ref`): иначе исполнитель не узнает,
/// что её стоит опросить снова, и задача уснёт навсегда.
pub struct CountDown {
    left: u32,
    polls: u32,
}

impl CountDown {
    pub const fn new(n: u32) -> Self {
        CountDown { left: n, polls: 0 }
    }
}

impl Future for CountDown {
    /// Сколько раз future опросили, включая последний, готовый опрос.
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        // CountDown: Unpin (все поля — числа), поэтому Pin<&mut Self>
        // свободно даёт &mut Self.
        self.polls += 1;
        if self.left == 0 {
            return Poll::Ready(self.polls);
        }
        self.left -= 1;
        cx.waker().wake_by_ref(); // «опроси меня ещё раз»
        Poll::Pending
    }
}
// ANCHOR_END: countdown

// ANCHOR: yield_now
/// Уступить исполнителю один раз: первый опрос — `Pending` с немедленным
/// пробуждением, второй — `Ready`. Минимальная точка приостановки, на
/// которой async fn превращается в автомат с состояниями (22.2).
pub fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

pub struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            return Poll::Ready(());
        }
        self.yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
// ANCHOR_END: yield_now

// ANCHOR: poll_count
/// Опрашивать future до готовности «пустым» будильником (`Waker::noop`) и
/// вернуть результат и число опросов. Годится только для future, которые
/// будят себя сами, — как `CountDown`: noop-будильник никого не разбудит.
pub fn poll_to_end<F: Future>(fut: F) -> (F::Output, u32) {
    let mut fut = core::pin::pin!(fut);
    let mut cx = Context::from_waker(core::task::Waker::noop());
    let mut polls = 0;
    loop {
        polls += 1;
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return (v, polls);
        }
    }
}
// ANCHOR_END: poll_count

// ANCHOR: block_on
/// Простейший исполнитель: опрашивать future на текущем потоке, а между
/// опросами спать в `thread::park`. Будильник — `Arc` с ручкой потока:
/// `wake` делает `unpark`, и поток просыпается для следующего опроса.
#[cfg(feature = "std")]
pub fn block_on<F: Future>(fut: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Wake, Waker};
    use std::thread::{self, Thread};

    struct ThreadWaker(Thread);

    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut fut = core::pin::pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            // park может проснуться и без unpark — тогда просто опросим
            // future ещё раз: лишний опрос безвреден.
            Poll::Pending => thread::park(),
        }
    }
}
// ANCHOR_END: block_on

// ANCHOR: thread_timer
/// Таймер на отдельном потоке: future хранит копию будильника, а поток по
/// истечении срока ставит флаг и будит задачу. Так `Waker` пересекает
/// границу потоков — поэтому он обязан быть `Send + Sync`.
#[cfg(feature = "std")]
pub struct ThreadTimer {
    shared: std::sync::Arc<std::sync::Mutex<TimerState>>,
}

#[cfg(feature = "std")]
struct TimerState {
    done: bool,
    waker: Option<core::task::Waker>,
}

#[cfg(feature = "std")]
impl ThreadTimer {
    pub fn new(after: std::time::Duration) -> Self {
        use std::sync::{Arc, Mutex};
        let shared = Arc::new(Mutex::new(TimerState { done: false, waker: None }));
        let theirs = Arc::clone(&shared);
        std::thread::spawn(move || {
            std::thread::sleep(after);
            let mut st = theirs.lock().unwrap();
            st.done = true;
            if let Some(w) = st.waker.take() {
                w.wake(); // разбудить задачу из чужого потока
            }
        });
        ThreadTimer { shared }
    }
}

#[cfg(feature = "std")]
impl Future for ThreadTimer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut st = self.shared.lock().unwrap();
        if st.done {
            return Poll::Ready(());
        }
        // Запомнить будильник *этого* опроса: задача могла переехать к
        // другому исполнителю, и старый будильник разбудил бы не того.
        st.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}
// ANCHOR_END: thread_timer

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_needs_n_plus_one_polls() {
        assert_eq!(poll_to_end(CountDown::new(0)), (1, 1));
        assert_eq!(poll_to_end(CountDown::new(3)), (4, 4));
    }

    #[test]
    fn yield_now_pends_exactly_once() {
        assert_eq!(poll_to_end(yield_now()), ((), 2));
    }

    #[test]
    fn block_on_drives_self_waking_future() {
        assert_eq!(block_on(CountDown::new(5)), 6);
    }

    #[test]
    #[cfg_attr(miri, ignore = "поток со sleep под Miri идёт долго")]
    fn thread_timer_wakes_from_another_thread() {
        let t0 = std::time::Instant::now();
        block_on(ThreadTimer::new(std::time::Duration::from_millis(20)));
        assert!(t0.elapsed() >= std::time::Duration::from_millis(20));
    }
}
