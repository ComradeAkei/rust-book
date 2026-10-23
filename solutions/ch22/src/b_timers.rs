//! Задача B: однопоточный исполнитель с таймерами.
//!
//! Очередь готовых задач, как в разделе 22.4, плюс таймеры: куча
//! `BinaryHeap` по сроку. Время — виртуальное: когда готовых задач нет,
//! исполнитель «перематывает» часы на ближайший срок и будит всех, чей срок
//! настал. Так поведение детерминировано, а тысяча таймеров по минуте
//! проходит за миллисекунды — тот же приём, что в симуляторах и в
//! `tokio::time::pause`. Будильник задачи — `Arc` с её номером.
//!
//! Отмена: `Sleep`, уроненный раньше срока (проиграл `select`), не должен
//! ни будить задачу, ни двигать часы. Таймер в куче и `Sleep` делят ячейку
//! с будильником; `Drop` у `Sleep` её опустошает, и исполнитель выбрасывает
//! такой таймер, не перематывая на него время.

use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

// ANCHOR: solution
type Task = Pin<Box<dyn Future<Output = ()>>>;
type Queue = Arc<Mutex<VecDeque<usize>>>;

struct TaskWaker {
    id: usize,
    queue: Queue,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.queue.lock().unwrap().push_back(self.id);
    }
}

/// Ячейка будильника, общая у таймера в куче и у `Sleep`. `None` — таймер
/// сработал или отменён.
type Slot = Rc<RefCell<Option<Waker>>>;

/// Таймер в куче: срок и порядковый номер (чтобы равные сроки срабатывали
/// в порядке постановки), будильник — отдельно, он не сравним.
struct Timer {
    deadline: u64,
    seq: u64,
    slot: Slot,
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        (self.deadline, self.seq) == (other.deadline, other.seq)
    }
}
impl Eq for Timer {}
impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.deadline, self.seq).cmp(&(other.deadline, other.seq))
    }
}

struct Inner {
    now: Cell<u64>,
    seq: Cell<u64>,
    timers: RefCell<BinaryHeap<Reverse<Timer>>>, // мин-куча по сроку
    tasks: RefCell<Vec<Option<Task>>>,
    spawned: RefCell<Vec<Task>>, // поставленные изнутри задач, до учёта
    queue: Queue,
}

/// Ручка исполнителя: ставить задачи, спать, узнавать время. Клонируется
/// в задачи — они живут на том же потоке, поэтому хватает `Rc`.
#[derive(Clone)]
pub struct Handle(Rc<Inner>);

pub struct Runtime {
    handle: Handle,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            handle: Handle(Rc::new(Inner {
                now: Cell::new(0),
                seq: Cell::new(0),
                timers: RefCell::new(BinaryHeap::new()),
                tasks: RefCell::new(Vec::new()),
                spawned: RefCell::new(Vec::new()),
                queue: Arc::default(),
            })),
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static) {
        self.handle.spawn(fut);
    }

    /// Исполнять, пока есть задачи; вернуть виртуальное время в конце.
    pub fn run(&self) -> u64 {
        let inner = &self.handle.0;
        loop {
            // 1. Учесть задачи, поставленные с прошлого шага.
            for task in inner.spawned.borrow_mut().drain(..) {
                let mut tasks = inner.tasks.borrow_mut();
                inner.queue.lock().unwrap().push_back(tasks.len());
                tasks.push(Some(task));
            }
            // 2. Опросить готовую задачу, если есть.
            let next = inner.queue.lock().unwrap().pop_front();
            if let Some(id) = next {
                // Вынуть задачу на время опроса: внутри poll она вправе
                // звать spawn и sleep, которые тоже трогают исполнитель.
                let Some(mut task) = inner.tasks.borrow_mut()[id].take() else {
                    continue; // завершена, а будильник сработал повторно
                };
                let waker =
                    Waker::from(Arc::new(TaskWaker { id, queue: Arc::clone(&inner.queue) }));
                if task.as_mut().poll(&mut Context::from_waker(&waker)).is_pending() {
                    inner.tasks.borrow_mut()[id] = Some(task);
                }
                continue;
            }
            if !inner.spawned.borrow().is_empty() {
                continue;
            }
            // 3. Готовых нет: перемотать часы на ближайший живой срок и
            //    разбудить всех, чей срок настал. Отменённые таймеры
            //    выбросить, не двигая на них часы. Таймеров нет — всё.
            let mut timers = inner.timers.borrow_mut();
            while timers.peek().is_some_and(|Reverse(t)| t.slot.borrow().is_none()) {
                timers.pop();
            }
            let Some(Reverse(first)) = timers.peek() else {
                return inner.now.get();
            };
            let at = first.deadline;
            inner.now.set(at);
            let mut due = Vec::new();
            while timers.peek().is_some_and(|Reverse(t)| t.deadline <= at) {
                let Reverse(t) = timers.pop().unwrap();
                due.extend(t.slot.borrow_mut().take());
            }
            drop(timers);
            due.into_iter().for_each(Waker::wake); // будим, отпустив кучу
        }
    }
}

impl Handle {
    pub fn now(&self) -> u64 {
        self.0.now.get()
    }

    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static) {
        self.0.spawned.borrow_mut().push(Box::pin(fut));
    }

    /// Уснуть на `ms` виртуальных миллисекунд.
    pub fn sleep(&self, ms: u64) -> Sleep {
        Sleep { handle: self.clone(), deadline: self.now() + ms, slot: None }
    }
}

pub struct Sleep {
    handle: Handle,
    deadline: u64,
    slot: Option<Slot>, // Some — таймер стоит в куче
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.handle.now() >= self.deadline {
            return Poll::Ready(());
        }
        match &self.slot {
            // Уже стоим в куче: обновить будильник — нас могли перенести
            // в другую задачу или опросить из другого select.
            Some(slot) => {
                let mut w = slot.borrow_mut();
                if !w.as_ref().is_some_and(|w| w.will_wake(cx.waker())) {
                    *w = Some(cx.waker().clone());
                }
            }
            None => {
                let slot: Slot = Rc::new(RefCell::new(Some(cx.waker().clone())));
                let inner = &self.handle.0;
                let seq = inner.seq.get();
                inner.seq.set(seq + 1);
                inner.timers.borrow_mut().push(Reverse(Timer {
                    deadline: self.deadline,
                    seq,
                    slot: Rc::clone(&slot),
                }));
                self.slot = Some(slot);
            }
        }
        Poll::Pending
    }
}

impl Drop for Sleep {
    /// Отмена: опустошить ячейку, чтобы таймер не будил и не двигал часы.
    fn drop(&mut self) {
        if let Some(slot) = &self.slot {
            slot.borrow_mut().take();
        }
    }
}
// ANCHOR_END: solution
