//! Задача C: реактор на epoll и эхо-сервер на одном потоке (Linux, x86-64).
//! Условие — раздел 22.7.
//!
//! Реактор — вторая половина асинхронного рантайма. Исполнитель опрашивает
//! задачи, а реактор знает, *когда* их опрашивать: задача, получившая
//! `WouldBlock`, оставляет реактору будильник и интерес к дескриптору
//! (читать или писать), а исполнитель, когда готовых задач нет, засыпает в
//! `epoll_wait` и будит тех, чьи дескрипторы ожили. Так один поток держит
//! сотни соединений. Системные вызовы — прямой `syscall`, как `futex` в
//! задаче C главы 20; крейт libc не нужен.
//!
//! Дескрипторы ставятся с `EPOLLONESHOT`: сработав один раз, дескриптор
//! выключается, пока задача снова не попросит, — у каждого дескриптора не
//! больше одного ждущего, и событие не будит дважды.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, RawFd};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

// ANCHOR: syscalls
const SYS_CLOSE: u64 = 3;
const SYS_EPOLL_WAIT: u64 = 232;
const SYS_EPOLL_CTL: u64 = 233;
const SYS_EPOLL_CREATE1: u64 = 291;

const EPOLL_CLOEXEC: u64 = 0o2_000_000;
const EPOLL_CTL_ADD: u64 = 1;
const EPOLL_CTL_DEL: u64 = 2;
const EPOLL_CTL_MOD: u64 = 3;
pub const EPOLLIN: u32 = 0x001;
pub const EPOLLOUT: u32 = 0x004;
const EPOLLONESHOT: u32 = 1 << 30;

/// `struct epoll_event` ядра. На x86-64 она упакована (`__EPOLL_PACKED`):
/// 12 байт, `data` по смещению 4. Без `packed` ядро и мы разошлись бы в
/// раскладке массива событий.
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct EpollEvent {
    events: u32,
    data: u64, // кладём сюда номер дескриптора
}

/// Системный вызов с четырьмя аргументами. Ядро возвращает `-errno` в
/// диапазоне -4095..-1 — переводим в `io::Error`.
///
/// # Safety
///
/// Аргументы должны удовлетворять контракту вызова `nr` (указатели живы и
/// достаточной длины).
unsafe fn syscall4(nr: u64, a1: u64, a2: u64, a3: u64, a4: u64) -> io::Result<u64> {
    let ret: i64;
    // SAFETY: соглашение Linux x86-64: номер в rax, аргументы в
    // rdi/rsi/rdx/r10; rcx и r11 портит сама инструкция syscall.
    unsafe {
        core::arch::asm!(
            "syscall",
            inlateout("rax") nr as i64 => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    if (-4095..0).contains(&ret) {
        Err(io::Error::from_raw_os_error(-ret as i32))
    } else {
        Ok(ret as u64)
    }
}

fn epoll_create() -> io::Result<RawFd> {
    // SAFETY: epoll_create1 не принимает указателей.
    unsafe { syscall4(SYS_EPOLL_CREATE1, EPOLL_CLOEXEC, 0, 0, 0).map(|fd| fd as RawFd) }
}

fn epoll_ctl(epfd: RawFd, op: u64, fd: RawFd, events: u32) -> io::Result<()> {
    let mut ev = EpollEvent { events, data: fd as u64 };
    // SAFETY: ev жива на время вызова; для DEL ядро указатель не читает.
    unsafe {
        syscall4(SYS_EPOLL_CTL, epfd as u64, op, fd as u64, &raw mut ev as u64)?;
    }
    Ok(())
}

/// Ждать событий без тайм-аута; вернуть, сколько записей заполнено.
fn epoll_wait(epfd: RawFd, events: &mut [EpollEvent]) -> io::Result<usize> {
    // SAFETY: ядро пишет не больше events.len() записей в живой срез.
    unsafe {
        let n = syscall4(
            SYS_EPOLL_WAIT,
            epfd as u64,
            events.as_mut_ptr() as u64,
            events.len() as u64,
            -1i64 as u64, // ждать сколько угодно
        )?;
        Ok(n as usize)
    }
}
// ANCHOR_END: syscalls

// ANCHOR: reactor
/// Реактор: экземпляр epoll и будильники ждущих задач. Ключ — дескриптор;
/// значение `Some(waker)` — задача ждёт, `None` — дескриптор в epoll, но
/// сейчас выключен (ONESHOT сработал или ещё никто не ждал).
pub struct Reactor {
    epfd: RawFd,
    wakers: RefCell<HashMap<RawFd, Option<Waker>>>,
}

impl Reactor {
    pub fn new() -> io::Result<Rc<Reactor>> {
        Ok(Rc::new(Reactor { epfd: epoll_create()?, wakers: RefCell::default() }))
    }

    /// Сколько задач сейчас ждёт событий.
    fn waiting(&self) -> usize {
        self.wakers.borrow().values().filter(|w| w.is_some()).count()
    }

    /// Включить интерес `events` к `fd` и запомнить будильник: впервые
    /// увиденный дескриптор — `EPOLL_CTL_ADD`, известный — `EPOLL_CTL_MOD`;
    /// к интересу всегда добавлять `EPOLLONESHOT`.
    fn arm(&self, fd: RawFd, events: u32, waker: &Waker) -> io::Result<()> {
        todo!("раздел 22.7, задача C")
    }

    /// Ждёт ли ещё задача на `fd` (в таблице `Some`), и если да — обновить
    /// её будильник, когда он сменился (`Waker::will_wake`).
    fn still_waiting(&self, fd: RawFd, waker: &Waker) -> bool {
        todo!("раздел 22.7, задача C")
    }

    fn deregister(&self, fd: RawFd) {
        if self.wakers.borrow_mut().remove(&fd).is_some() {
            // Ошибку не поднимаем: дескриптор и так вот-вот закроется.
            let _ = epoll_ctl(self.epfd, EPOLL_CTL_DEL, fd, 0);
        }
    }

    /// Уснуть в ядре до первого события и разбудить ожившие задачи:
    /// `epoll_wait` (повторить на `Interrupted`), для каждого события взять
    /// будильник из таблицы (`Option::take`) и разбудить — уже отпустив
    /// `RefCell`. Вернуть число событий.
    fn wait(&self) -> io::Result<usize> {
        todo!("раздел 22.7, задача C")
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        // SAFETY: epfd принадлежит реактору и больше нигде не используется.
        let _ = unsafe { syscall4(SYS_CLOSE, self.epfd as u64, 0, 0, 0) };
    }
}

/// Future «дескриптор готов к `events`». Первый опрос включает интерес и
/// возвращает `Pending`; следующие — `Ready`, когда реактор разбудил.
struct Ready<'a> {
    reactor: &'a Reactor,
    fd: RawFd,
    events: u32,
    armed: bool,
}

impl Future for Ready<'_> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        todo!("раздел 22.7, задача C")
    }
}

/// Неблокирующий ресурс под присмотром реактора: попытаться операцию,
/// на `WouldBlock` дождаться готовности и повторить. Так устроен крейт
/// async-io (smol).
pub struct Async<T: AsRawFd> {
    io: T,
    reactor: Rc<Reactor>,
}

impl<T: AsRawFd> Async<T> {
    /// `io` должен быть уже переведён в неблокирующий режим.
    pub fn new(io: T, reactor: &Rc<Reactor>) -> Self {
        Async { io, reactor: Rc::clone(reactor) }
    }

    pub fn get_ref(&self) -> &T {
        &self.io
    }

    async fn with<R>(&self, events: u32, mut op: impl FnMut(&T) -> io::Result<R>) -> io::Result<R> {
        loop {
            match op(&self.io) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    let fd = self.io.as_raw_fd();
                    Ready { reactor: &self.reactor, fd, events, armed: false }.await?;
                }
                r => return r,
            }
        }
    }

    pub async fn read_with<R>(&self, op: impl FnMut(&T) -> io::Result<R>) -> io::Result<R> {
        self.with(EPOLLIN, op).await
    }

    pub async fn write_with<R>(&self, op: impl FnMut(&T) -> io::Result<R>) -> io::Result<R> {
        self.with(EPOLLOUT, op).await
    }
}

impl<T: AsRawFd> Drop for Async<T> {
    /// Выписать дескриптор из epoll до того, как поле `io` его закроет.
    fn drop(&mut self) {
        self.reactor.deregister(self.io.as_raw_fd());
    }
}
// ANCHOR_END: reactor

// ANCHOR: runtime
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

/// Исполнитель из 22.4 плюс реактор: когда готовых задач нет, он не
/// выходит, а спит в `epoll_wait`.
pub struct Runtime {
    reactor: Rc<Reactor>,
    tasks: RefCell<Vec<Option<Task>>>,
    spawned: RefCell<Vec<Task>>,
    queue: Queue,
}

impl Runtime {
    pub fn new() -> io::Result<Rc<Runtime>> {
        Ok(Rc::new(Runtime {
            reactor: Reactor::new()?,
            tasks: RefCell::default(),
            spawned: RefCell::default(),
            queue: Arc::default(),
        }))
    }

    pub fn reactor(&self) -> &Rc<Reactor> {
        &self.reactor
    }

    /// Поставить задачу; можно звать и изнутри задач.
    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static) {
        self.spawned.borrow_mut().push(Box::pin(fut));
    }

    /// Исполнять, пока живы задачи. Ошибка — если задачи ждут, а будить
    /// их некому: ни очереди, ни интереса в epoll.
    pub fn run(&self) -> io::Result<()> {
        loop {
            for task in self.spawned.borrow_mut().drain(..) {
                let mut tasks = self.tasks.borrow_mut();
                self.queue.lock().unwrap().push_back(tasks.len());
                tasks.push(Some(task));
            }
            let next = self.queue.lock().unwrap().pop_front();
            if let Some(id) = next {
                let Some(mut task) = self.tasks.borrow_mut()[id].take() else {
                    continue;
                };
                let waker = Waker::from(Arc::new(TaskWaker { id, queue: Arc::clone(&self.queue) }));
                if task.as_mut().poll(&mut Context::from_waker(&waker)).is_pending() {
                    self.tasks.borrow_mut()[id] = Some(task);
                }
                continue;
            }
            if !self.spawned.borrow().is_empty() {
                continue;
            }
            if self.tasks.borrow().iter().all(Option::is_none) {
                return Ok(());
            }
            if self.reactor.waiting() == 0 {
                return Err(io::Error::other("задачи ждут, но разбудить их некому"));
            }
            self.reactor.wait()?;
        }
    }
}
// ANCHOR_END: runtime

// ANCHOR: server
/// Итоги работы сервера.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub accepted: usize,
    pub bytes: usize,
    pub errors: usize,
    /// Наибольшее число одновременно живых соединений.
    pub peak: usize,
}

#[derive(Default)]
struct Counters {
    accepted: Cell<usize>,
    bytes: Cell<usize>,
    errors: Cell<usize>,
    live: Cell<usize>,
    peak: Cell<usize>,
}

/// Эхо одного соединения: читать (`read_with`), пока клиент не закроет
/// запись (прочитано 0), и отправлять прочитанное обратно (`write_with`;
/// `write` может взять не всё). Прибавлять отправленное к `counters.bytes`.
async fn echo(stream: Async<TcpStream>, counters: Rc<Counters>) -> io::Result<()> {
    todo!("раздел 22.7, задача C")
}

/// Принять `conns` соединений на `listener` и обслужить их все на текущем
/// потоке. Возвращается, когда последнее соединение закрыто.
pub fn serve(listener: TcpListener, conns: usize) -> io::Result<Stats> {
    listener.set_nonblocking(true)?;
    let rt = Runtime::new()?;
    let counters = Rc::new(Counters::default());
    let listener = Async::new(listener, rt.reactor());
    {
        let rt2 = Rc::clone(&rt);
        let counters = Rc::clone(&counters);
        rt.spawn(async move {
            for _ in 0..conns {
                let stream = match listener.read_with(|l| l.accept()).await {
                    Ok((s, _)) => s,
                    Err(_) => {
                        counters.errors.set(counters.errors.get() + 1);
                        continue;
                    }
                };
                if stream.set_nonblocking(true).is_err() {
                    counters.errors.set(counters.errors.get() + 1);
                    continue;
                }
                counters.accepted.set(counters.accepted.get() + 1);
                counters.live.set(counters.live.get() + 1);
                counters.peak.set(counters.peak.get().max(counters.live.get()));
                let stream = Async::new(stream, rt2.reactor());
                let counters = Rc::clone(&counters);
                rt2.spawn(async move {
                    if echo(stream, Rc::clone(&counters)).await.is_err() {
                        counters.errors.set(counters.errors.get() + 1);
                    }
                    counters.live.set(counters.live.get() - 1);
                });
            }
        });
    }
    rt.run()?;
    Ok(Stats {
        accepted: counters.accepted.get(),
        bytes: counters.bytes.get(),
        errors: counters.errors.get(),
        peak: counters.peak.get(),
    })
}
// ANCHOR_END: server

// ANCHOR: threads
/// Для сравнения: тот же эхо-сервер по потоку на соединение, с
/// блокирующим вводом-выводом. Код проще, но каждое соединение — это стек
/// потока (по умолчанию 2 МиБ виртуальной памяти) и структура задачи ядра.
pub fn serve_threads(listener: TcpListener, conns: usize) -> io::Result<Stats> {
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    let (bytes, errors, live, peak) =
        (AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0));
    let accepted = std::thread::scope(|s| {
        let mut accepted = 0;
        for _ in 0..conns {
            let Ok((mut stream, _)) = listener.accept() else {
                errors.fetch_add(1, Relaxed);
                continue;
            };
            accepted += 1;
            peak.fetch_max(live.fetch_add(1, Relaxed) + 1, Relaxed);
            let (bytes, errors, live) = (&bytes, &errors, &live);
            s.spawn(move || {
                let mut buf = [0u8; 4096];
                let res = (|| -> io::Result<()> {
                    loop {
                        let n = stream.read(&mut buf)?;
                        if n == 0 {
                            return Ok(());
                        }
                        stream.write_all(&buf[..n])?;
                        bytes.fetch_add(n, Relaxed);
                    }
                })();
                if res.is_err() {
                    errors.fetch_add(1, Relaxed);
                }
                live.fetch_sub(1, Relaxed);
            });
        }
        accepted
    });
    Ok(Stats {
        accepted,
        bytes: bytes.into_inner(),
        errors: errors.into_inner(),
        peak: peak.into_inner(),
    })
}
// ANCHOR_END: threads
