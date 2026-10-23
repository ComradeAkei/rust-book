//! Раздел 22.4: исполнитель с очередью задач.
//!
//! Исполнитель хранит задачи — закреплённые в куче future — и очередь
//! номеров тех, кого пора опросить. Будильник задачи знает её номер и по
//! `wake` кладёт его в очередь. Исполнитель вынимает номер, опрашивает
//! задачу и, если она вернула `Pending`, забывает о ней до следующего
//! пробуждения. Задачи здесь не `Send` (живут на одном потоке), но сам
//! `Waker` обязан быть `Send + Sync`: его вправе разбудить кто угодно и
//! откуда угодно — другой поток, обработчик сигнала, реактор.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

// ANCHOR: executor
type Task = Pin<Box<dyn Future<Output = ()>>>;
type Queue = Arc<Mutex<VecDeque<usize>>>;

/// Будильник одной задачи: «положи мой номер в очередь».
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

#[derive(Default)]
pub struct Executor {
    tasks: Vec<Option<Task>>, // None — задача завершилась
    queue: Queue,
    /// Сколько раз опрашивали задачи — для наглядности в тестах и тексте.
    pub polls: usize,
}

impl Executor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Поставить задачу: она сразу попадает в очередь на первый опрос.
    pub fn spawn(&mut self, fut: impl Future<Output = ()> + 'static) {
        let id = self.tasks.len();
        self.tasks.push(Some(Box::pin(fut)));
        self.queue.lock().unwrap().push_back(id);
    }

    /// Опрашивать задачи, пока в очереди есть кто-то. Вернуть число
    /// незавершённых задач: если оно не ноль, их некому разбудить —
    /// у настоящего исполнителя здесь ждал бы реактор (epoll, прерывание).
    pub fn run(&mut self) -> usize {
        loop {
            // Не держать замок очереди во время poll: задача сама может
            // разбудить себя (или соседа) и захотеть положить номер.
            let next = self.queue.lock().unwrap().pop_front();
            let Some(id) = next else { break };
            let Some(task) = self.tasks[id].as_mut() else {
                continue; // разбудили уже завершённую задачу — пропустить
            };
            let waker = Waker::from(Arc::new(TaskWaker { id, queue: Arc::clone(&self.queue) }));
            let mut cx = Context::from_waker(&waker);
            self.polls += 1;
            if let Poll::Ready(()) = task.as_mut().poll(&mut cx) {
                self.tasks[id] = None; // освободить future и всё её состояние
            }
        }
        self.tasks.iter().filter(|t| t.is_some()).count()
    }
}
// ANCHOR_END: executor

#[cfg(test)]
mod tests {
    use super::*;
    use crate::future::{CountDown, yield_now};
    use std::cell::RefCell;
    use std::rc::Rc;

    // ANCHOR: interleave
    #[test]
    fn tasks_interleave_at_await_points() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut ex = Executor::new();
        for name in ["a", "b"] {
            let log = Rc::clone(&log);
            ex.spawn(async move {
                for i in 0..3 {
                    log.borrow_mut().push(format!("{name}{i}"));
                    yield_now().await; // уступить соседу
                }
            });
        }
        assert_eq!(ex.run(), 0);
        assert_eq!(*log.borrow(), ["a0", "b0", "a1", "b1", "a2", "b2"]);
        assert_eq!(ex.polls, 8); // по 4 опроса: 3 уступки + завершение
    }
    // ANCHOR_END: interleave

    #[test]
    fn pending_task_without_waker_is_reported() {
        struct Never;
        impl Future for Never {
            type Output = ();
            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
                Poll::Pending // никого не будит
            }
        }
        let mut ex = Executor::new();
        ex.spawn(Never);
        ex.spawn(async {
            CountDown::new(2).await;
        });
        assert_eq!(ex.run(), 1); // одна задача уснула навсегда
    }
}
