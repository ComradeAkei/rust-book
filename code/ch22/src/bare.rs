//! Раздел 22.6: исполнитель без кучи и без ОС.
//!
//! Future не требует ни кучи, ни потоков: это просто значение с методом
//! `poll`. Значит, async работает и на микроконтроллере. Нужны три вещи:
//! место для future (стек `block_on` или `static`), будильник без кучи
//! (`RawWaker` с собственной таблицей функций) и способ спать, пока будить
//! некому, — на Cortex-M это `wfi`, «ждать прерывания».
//!
//! Будильник здесь — указатель на флаг готовности исполнителя. `wake` ставит
//! флаг; обработчик прерывания будит исполнитель тем же способом. Все
//! операции — атомарные load/store без read-modify-write, поэтому модуль
//! собирается и на Cortex-M0 (глава 21).

use core::future::Future;
use core::pin::pin;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// ANCHOR: waker
/// Таблица будильника: данные — адрес `&'static AtomicBool` флага
/// готовности. Клон копирует адрес, `wake` ставит флаг, `drop` — ничего:
/// владеть нечем, флаг статический.
static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, noop_drop);

fn clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

fn wake(data: *const ()) {
    // SAFETY: data всегда получен из &'static AtomicBool в `waker`.
    let ready = unsafe { &*(data as *const AtomicBool) };
    ready.store(true, Ordering::Release);
}

fn noop_drop(_: *const ()) {}

/// Будильник, который ставит `ready`. Без кучи и без счётчика ссылок.
pub fn waker(ready: &'static AtomicBool) -> Waker {
    let raw = RawWaker::new(ready as *const AtomicBool as *const (), &VTABLE);
    // SAFETY: функции VTABLE соблюдают контракт RawWaker: data — указатель
    // на статический AtomicBool, живущий вечно; clone/wake/drop потокобезопасны.
    unsafe { Waker::from_raw(raw) }
}
// ANCHOR_END: waker

// ANCHOR: block_on
/// Что сделал `block_on`: результат, сколько раз опросил future и сколько
/// раз уходил в сон.
pub struct Run<T> {
    pub output: T,
    pub polls: u32,
    pub idles: u32,
}

/// Исполнить одну future до готовности. `ready` — флаг этого исполнителя,
/// его ставят будильник и обработчики прерываний. `idle` вызывается, когда
/// будить некому: на Cortex-M это [`cortex_idle`], в тестах — имитация
/// пришедшего прерывания.
pub fn block_on<F: Future>(
    ready: &'static AtomicBool,
    fut: F,
    mut idle: impl FnMut(&'static AtomicBool),
) -> Run<F::Output> {
    let mut fut = pin!(fut); // future живёт на стеке block_on, без кучи
    let waker = waker(ready);
    let mut cx = Context::from_waker(&waker);
    let (mut polls, mut idles) = (0, 0);
    ready.store(true, Ordering::Relaxed); // первый опрос — сразу
    loop {
        if ready.load(Ordering::Acquire) {
            // Сбросить флаг ДО опроса: пробуждение во время poll поставит
            // его снова, и мы опросим ещё раз, а не уснём.
            ready.store(false, Ordering::Relaxed);
            polls += 1;
            if let Poll::Ready(output) = fut.as_mut().poll(&mut cx) {
                return Run { output, polls, idles };
            }
        } else {
            idles += 1;
            idle(ready);
        }
    }
}
// ANCHOR_END: block_on

// ANCHOR: ticks
/// Счётчик событий из прерывания (тиков таймера, принятых байтов). Пишет
/// его только обработчик прерывания, читает только задача: один писатель —
/// значит, хватает load/store, CAS не нужен (как в SPSC из 21.5).
pub struct Ticks {
    count: AtomicU32,
    ready: &'static AtomicBool, // флаг исполнителя, которого будить
}

impl Ticks {
    pub const fn new(ready: &'static AtomicBool) -> Self {
        Ticks { count: AtomicU32::new(0), ready }
    }

    /// Вызывать из обработчика прерывания: отметить событие и разбудить
    /// исполнитель. Release: задача, увидев новое значение, увидит и всё,
    /// что обработчик записал до него.
    pub fn tick(&self) {
        let n = self.count.load(Ordering::Relaxed);
        self.count.store(n.wrapping_add(1), Ordering::Release);
        self.ready.store(true, Ordering::Release);
    }

    /// Дождаться события, ещё не виденного задачей. `seen` — сколько событий
    /// задача уже обработала; хранится у неё, а не в общем счётчике.
    pub fn next<'a>(&'a self, seen: &'a mut u32) -> NextTick<'a> {
        NextTick { ticks: self, seen }
    }
}

pub struct NextTick<'a> {
    ticks: &'a Ticks,
    seen: &'a mut u32,
}

impl Future for NextTick<'_> {
    type Output = u32;

    fn poll(mut self: core::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        // Будильник не сохраняем: обработчик знает флаг исполнителя сам.
        // Это допустимо, когда исполнитель один на ядро, — как здесь.
        if self.ticks.count.load(Ordering::Acquire) != *self.seen {
            *self.seen = self.seen.wrapping_add(1);
            Poll::Ready(*self.seen)
        } else {
            Poll::Pending
        }
    }
}
// ANCHOR_END: ticks

// ANCHOR: cortex_idle
/// Сон до прерывания без потерянного пробуждения. Проверить флаг и уснуть
/// нужно атомарно относительно прерываний: иначе прерывание, пришедшее
/// между проверкой и `wfi`, поставит флаг, вернётся — и ядро уснёт, хотя
/// работа есть. Поэтому: запретить прерывания (`cpsid i`), проверить флаг,
/// `wfi` — он просыпается и от *отложенного* прерывания, даже при запрете,
/// — и разрешить (`cpsie i`): обработчик выполнится сразу после.
#[cfg(target_arch = "arm")]
pub fn cortex_idle(ready: &'static AtomicBool) {
    // SAFETY: cpsid/cpsie меняют только PRIMASK; wfi лишь ждёт прерывания.
    // Порядок памяти обеспечивает Acquire-чтение флага внутри секции.
    unsafe {
        core::arch::asm!("cpsid i", options(nomem, nostack, preserves_flags));
        if !ready.load(Ordering::Acquire) {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
        core::arch::asm!("cpsie i", options(nomem, nostack, preserves_flags));
    }
}
// ANCHOR_END: cortex_idle

#[cfg(test)]
mod tests {
    use super::*;

    // ANCHOR: host_test
    #[test]
    fn ticks_from_simulated_interrupt() {
        static READY: AtomicBool = AtomicBool::new(false);
        static TICKS: Ticks = Ticks::new(&READY);
        let run = block_on(
            &READY,
            async {
                let mut seen = 0;
                let mut sum = 0;
                for _ in 0..3 {
                    sum += TICKS.next(&mut seen).await;
                }
                sum
            },
            // «Сон»: пока ядро спит, приходит прерывание таймера.
            |_| TICKS.tick(),
        );
        assert_eq!(run.output, 1 + 2 + 3);
        assert_eq!((run.polls, run.idles), (4, 3)); // первый опрос + по тику
    }
    // ANCHOR_END: host_test

    #[test]
    fn self_waking_future_never_idles() {
        static READY: AtomicBool = AtomicBool::new(false);
        let run = block_on(&READY, crate::future::CountDown::new(5), |_| {
            panic!("CountDown будит себя сам — спать незачем")
        });
        assert_eq!((run.output, run.polls, run.idles), (6, 6, 0));
    }
}
