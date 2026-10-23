//! Задача D1: исполнитель без кучи для нескольких задач, которых будят
//! прерывания.
//!
//! Раздел 22.6 исполнял одну future. Здесь задач несколько, и у каждой
//! свой флаг готовности: будильник задачи — адрес её флага, обработчик
//! прерывания ставит флаг той задачи, чьё событие пришло, и исполнитель
//! опрашивает только её. Ни кучи, ни счётчиков ссылок, только load/store
//! атомиков — годится и для Cortex-M0.
//!
//! Задачи живут в арене: тип future безымянный, и в `static` его не
//! записать, поэтому арена хранит future как байты `'static`-буфера и
//! отдаёт наружу `Pin<&'static mut dyn Future>`. Так размещает задачи
//! embassy на stable.

use core::future::Future;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicUsize, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// ANCHOR: arena
/// Задача: закреплённая навсегда future без результата.
pub type Task = Pin<&'static mut dyn Future<Output = ()>>;

/// Арена задач: `'static`-буфер, из которого future нарезаются подряд и
/// не освобождаются никогда (задачи прошивки живут вечно).
pub struct Arena {
    rest: &'static mut [MaybeUninit<u8>],
}

impl Arena {
    pub fn new(buf: &'static mut [MaybeUninit<u8>]) -> Self {
        Arena { rest: buf }
    }

    /// Сколько байтов арены ещё свободно.
    pub fn remaining(&self) -> usize {
        self.rest.len()
    }

    /// Разместить future в арене; `None`, если она не влезает.
    pub fn alloc<F: Future<Output = ()> + 'static>(&mut self, fut: F) -> Option<Task> {
        let rest = core::mem::take(&mut self.rest);
        // Отступ до выравнивания F и место под саму F.
        let pad = rest.as_ptr().align_offset(align_of::<F>());
        if pad.checked_add(size_of::<F>()).is_none_or(|end| end > rest.len()) {
            self.rest = rest;
            return None;
        }
        let (_, tail) = rest.split_at_mut(pad);
        let (slot, tail) = tail.split_at_mut(size_of::<F>());
        self.rest = tail; // slot отрезан навсегда: больше его никто не получит
        let p = slot.as_mut_ptr().cast::<F>();
        // SAFETY: slot — 'static-память, принадлежащая только нам (отрезана
        // от rest), выровнена под F и вмещает F; запись инициализирует её.
        let fut: &'static mut F = unsafe {
            p.write(fut);
            &mut *p
        };
        // 'static mut: значение не сдвинется и не будет уничтожено никогда —
        // ровно то, что обещает Pin. Поэтому закрепление безопасно.
        Some(Pin::static_mut(fut))
    }
}
// ANCHOR_END: arena

// ANCHOR: executor
/// Будильник задачи — адрес её флага готовности. Клон копирует адрес,
/// `wake` ставит флаг, `drop` — ничего: флаг статический.
static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, noop_drop);

fn clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

fn wake(data: *const ()) {
    // SAFETY: data всегда получен из &'static AtomicBool в flag_waker.
    unsafe { &*(data as *const AtomicBool) }.store(true, Ordering::Release);
}

fn noop_drop(_: *const ()) {}

pub fn flag_waker(flag: &'static AtomicBool) -> Waker {
    let raw = RawWaker::new(flag as *const AtomicBool as *const (), &VTABLE);
    // SAFETY: data — указатель на статический AtomicBool, функции VTABLE
    // только читают его адрес и делают атомарный store.
    unsafe { Waker::from_raw(raw) }
}

/// Что сделал исполнитель: опросы каждой задачи и уходы в сон.
#[derive(Debug, PartialEq, Eq)]
pub struct Report<const N: usize> {
    pub polls: [u32; N],
    pub idles: u32,
}

pub struct Executor<const N: usize> {
    flags: &'static [AtomicBool; N],
    tasks: [Option<Task>; N],
}

impl<const N: usize> Executor<N> {
    /// Задача `i` получает флаг `flags[i]`. Все флаги поднимаются: каждую
    /// задачу нужно опросить хотя бы раз.
    pub fn new(flags: &'static [AtomicBool; N], tasks: [Task; N]) -> Self {
        for f in flags {
            f.store(true, Ordering::Relaxed);
        }
        Executor { flags, tasks: tasks.map(Some) }
    }

    /// Опрашивать задачи с поднятым флагом, пока все не завершатся. Если
    /// за полный проход готовых не нашлось — вызвать `idle` (на Cortex-M он
    /// под запретом прерываний проверит флаги ещё раз и уснёт в `wfi`).
    pub fn run(&mut self, mut idle: impl FnMut(&'static [AtomicBool; N])) -> Report<N> {
        let mut report = Report { polls: [0; N], idles: 0 };
        loop {
            let mut polled = false;
            for (i, slot) in self.tasks.iter_mut().enumerate() {
                let flag = &self.flags[i];
                // Флаг сбрасываем ДО опроса: пробуждение во время poll
                // поднимет его снова, и мы не уснём с готовой задачей.
                if !flag.load(Ordering::Acquire) {
                    continue;
                }
                flag.store(false, Ordering::Relaxed);
                let Some(task) = slot.as_mut() else {
                    continue; // будильник завершённой задачи — только сбросить
                };
                polled = true;
                report.polls[i] += 1;
                let waker = flag_waker(flag);
                if task.as_mut().poll(&mut Context::from_waker(&waker)).is_ready() {
                    *slot = None;
                }
            }
            if self.tasks.iter().all(Option::is_none) {
                return report;
            }
            if !polled {
                report.idles += 1;
                idle(self.flags);
            }
        }
    }
}

/// Сон до прерывания для нескольких флагов: под `cpsid i` проверить, что
/// ни один не поднят, и только тогда `wfi` (см. `cortex_idle` в 22.6).
#[cfg(target_arch = "arm")]
pub fn cortex_idle<const N: usize>(flags: &'static [AtomicBool; N]) {
    // SAFETY: cpsid/cpsie меняют только PRIMASK; wfi лишь ждёт прерывания.
    unsafe {
        core::arch::asm!("cpsid i", options(nomem, nostack, preserves_flags));
        if !flags.iter().any(|f| f.load(Ordering::Acquire)) {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
        core::arch::asm!("cpsie i", options(nomem, nostack, preserves_flags));
    }
}
// ANCHOR_END: executor

// ANCHOR: events
/// Счётчик событий от прерывания (тиков таймера). Пишет только
/// обработчик, читает только задача — хватает load/store.
pub struct Signal {
    count: AtomicU32,
    task: &'static AtomicBool, // флаг задачи, которую будить
}

impl Signal {
    pub const fn new(task: &'static AtomicBool) -> Self {
        Signal { count: AtomicU32::new(0), task }
    }

    /// Из обработчика: отметить событие и разбудить задачу.
    pub fn raise(&self) {
        let n = self.count.load(Ordering::Relaxed);
        self.count.store(n.wrapping_add(1), Ordering::Release);
        self.task.store(true, Ordering::Release);
    }

    /// Дождаться события, которого задача ещё не видела.
    pub async fn wait(&self, seen: &mut u32) -> u32 {
        core::future::poll_fn(|_| {
            if self.count.load(Ordering::Acquire) == *seen {
                return Poll::Pending; // разбудит raise: флаг задачи он знает
            }
            *seen = seen.wrapping_add(1);
            Poll::Ready(*seen)
        })
        .await
    }
}

/// Очередь байтов из обработчика прерывания (приём UART) в задачу: кольцо
/// SPSC, как в 21.5, на атомарных индексах без RMW. Переполнение не
/// блокирует обработчик: лишний байт отбрасывается и считается.
pub struct ByteQueue<const N: usize> {
    buf: [AtomicU8; N],
    head: AtomicUsize, // пишет только обработчик
    tail: AtomicUsize, // пишет только задача
    lost: AtomicU32,
    task: &'static AtomicBool,
}

impl<const N: usize> ByteQueue<N> {
    /// `N` — степень двойки: индексы растут без сброса, и `% N` должен
    /// оставаться непрерывным при переполнении usize.
    pub const fn new(task: &'static AtomicBool) -> Self {
        const {
            assert!(N.is_power_of_two(), "размер очереди — степень двойки")
        };
        ByteQueue {
            buf: [const { AtomicU8::new(0) }; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            lost: AtomicU32::new(0),
            task,
        }
    }

    /// Из обработчика: положить байт и разбудить задачу.
    pub fn push(&self, byte: u8) {
        let head = self.head.load(Ordering::Relaxed);
        if head.wrapping_sub(self.tail.load(Ordering::Acquire)) == N {
            let lost = self.lost.load(Ordering::Relaxed);
            self.lost.store(lost + 1, Ordering::Relaxed);
        } else {
            self.buf[head % N].store(byte, Ordering::Relaxed);
            self.head.store(head.wrapping_add(1), Ordering::Release);
        }
        self.task.store(true, Ordering::Release);
    }

    /// Сколько байтов отброшено из-за переполнения.
    pub fn lost(&self) -> u32 {
        self.lost.load(Ordering::Relaxed)
    }

    /// Дождаться следующего байта.
    pub async fn pop(&self) -> u8 {
        core::future::poll_fn(|_| {
            let tail = self.tail.load(Ordering::Relaxed);
            if self.head.load(Ordering::Acquire) == tail {
                return Poll::Pending;
            }
            let b = self.buf[tail % N].load(Ordering::Relaxed);
            self.tail.store(tail.wrapping_add(1), Ordering::Release);
            Poll::Ready(b)
        })
        .await
    }
}
// ANCHOR_END: events
