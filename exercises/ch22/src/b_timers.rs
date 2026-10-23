//! Задача B: однопоточный исполнитель с таймерами. Условие — раздел 22.7.
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

// ANCHOR: skeleton
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
    ///
    /// Цикл: учесть задачи из `spawned`; если очередь не пуста — вынуть
    /// номер, вынуть задачу из `tasks` на время опроса (внутри poll она
    /// вправе звать `spawn` и `sleep`) и опросить с будильником `TaskWaker`;
    /// иначе выбросить отменённые таймеры (пустая ячейка), перемотать
    /// `now` на ближайший срок и разбудить всех, чей срок настал. Нет ни
    /// готовых задач, ни таймеров — вернуть `now`.
    pub fn run(&self) -> u64 {
        todo!("раздел 22.7, задача B")
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

    /// Срок настал — `Ready`. Иначе при первом опросе поставить таймер в
    /// кучу (срок, очередной `seq`, общая ячейка с будильником), при
    /// следующих — обновить будильник в ячейке, если он сменился.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        todo!("раздел 22.7, задача B")
    }
}

impl Drop for Sleep {
    /// Отмена: опустошить ячейку, чтобы таймер не будил и не двигал часы.
    fn drop(&mut self) {
        // Задача B: опустошите ячейку таймера, если он стоит в куче. Здесь
        // нет todo!(): паника в drop при раскрутке другой паники убила бы
        // весь процесс тестов.
    }
}
// ANCHOR_END: skeleton
