// Общие тесты главы 22 для задач A–D2: подключаются через include! в крейты
// заготовок и решений после строки `use <крейт> as sut;`.
//
// Задача A сверяет предсказания с замером — размеры автоматов у rustc 1.97
// на x86-64, поэтому её тест в группе `observed`. Задача C (epoll, сеть) —
// только Linux x86-64 и под Miri помечена ignore: Miri не исполняет asm.

use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll, Waker};

// --- Вспомогательное: опрос до конца, тайм-аут по опросам, select ---

/// Опрашивать future с пустым будильником до готовности; вернуть результат
/// и число опросов. Годится для future, которые будят себя сами.
fn poll_to_end<F: Future>(fut: F) -> (F::Output, u32) {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    let mut polls = 0;
    loop {
        polls += 1;
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return (v, polls);
        }
        assert!(polls < 10_000_000, "future не завершается");
    }
}

/// Тайм-аут: `Pending` `n` раз (будя себя), затем `Ready`.
struct Timeout(u32);

impl Future for Timeout {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 == 0 {
            return Poll::Ready(());
        }
        self.0 -= 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

enum Either<A, B> {
    Left(A),
    Right(B),
}

/// Первая готовая из двух; проигравшая роняется вместе с `Select`.
struct Select<A, B>(A, B);

impl<A: Future + Unpin, B: Future + Unpin> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(v) = Pin::new(&mut self.0).poll(cx) {
            return Poll::Ready(Either::Left(v));
        }
        if let Poll::Ready(v) = Pin::new(&mut self.1).poll(cx) {
            return Poll::Ready(Either::Right(v));
        }
        Poll::Pending
    }
}

// --- Задача A: размеры автоматов ---

#[test]
fn a_futures_compute_right_values() {
    use sut::a_sizes::*;
    let v = [3u8, 1, 4, 1, 5];
    assert_eq!(poll_to_end(s0()).0, 42);
    assert_eq!(poll_to_end(s1(7)).0, 7);
    assert_eq!(poll_to_end(s2()).0, 7 * 64);
    assert_eq!(poll_to_end(s3()).0, 7 * 64);
    assert_eq!(poll_to_end(s4()).0, 2 * 7 * 64);
    assert_eq!(poll_to_end(s5(&v)).0, 5 + 3);
}

#[cfg(target_pointer_width = "64")]
mod observed {
    // Раскладка автоматов async fn не стабилизирована: числа верны для
    // rustc 1.97 на x86-64 и могут измениться с версией компилятора.
    #[test]
    fn observed_a_predictions_match_measurement() {
        let m = super::sut::a_sizes::measured();
        assert_eq!(m, [1, 24, 66, 16, 88, 40], "замер изменился — новая версия rustc?");
        assert_eq!(super::sut::a_sizes::predicted(), m, "предсказания [s0..s5] против замера");
    }
}

// --- Задача B: исполнитель с таймерами ---

#[test]
fn b_timers_fire_in_deadline_order() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let rt = sut::b_timers::Runtime::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    for ms in [30, 10, 20] {
        let (h, log) = (rt.handle(), Rc::clone(&log));
        rt.spawn(async move {
            h.sleep(ms).await;
            log.borrow_mut().push((ms, h.now()));
        });
    }
    assert_eq!(rt.run(), 30, "часы стоят на последнем сроке");
    assert_eq!(*log.borrow(), [(10, 10), (20, 20), (30, 30)]);
}

#[test]
fn b_equal_deadlines_fire_in_registration_order() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let rt = sut::b_timers::Runtime::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    for name in ["a", "b", "c"] {
        let (h, log) = (rt.handle(), Rc::clone(&log));
        rt.spawn(async move {
            h.sleep(5).await;
            log.borrow_mut().push(name);
        });
    }
    assert_eq!(rt.run(), 5);
    assert_eq!(*log.borrow(), ["a", "b", "c"]);
}

#[test]
fn b_sleeps_add_up_and_tasks_spawn_tasks() {
    use std::cell::Cell;
    use std::rc::Rc;
    let rt = sut::b_timers::Runtime::new();
    let done = Rc::new(Cell::new(0));
    let (h, d) = (rt.handle(), Rc::clone(&done));
    rt.spawn(async move {
        h.sleep(5).await;
        h.sleep(7).await;
        assert_eq!(h.now(), 12);
        let (h2, d2) = (h.clone(), Rc::clone(&d));
        h.spawn(async move {
            h2.sleep(100).await;
            assert_eq!(h2.now(), 112);
            d2.set(d2.get() + 1);
        });
        d.set(d.get() + 1);
    });
    assert_eq!(rt.run(), 112);
    assert_eq!(done.get(), 2);
}

#[test]
fn b_zero_sleep_and_empty_runtime() {
    let rt = sut::b_timers::Runtime::new();
    assert_eq!(rt.run(), 0, "без задач время не идёт");
    let h = rt.handle();
    rt.spawn(async move {
        h.sleep(0).await; // срок уже настал
        assert_eq!(h.now(), 0);
    });
    assert_eq!(rt.run(), 0);
}

#[test]
#[cfg_attr(miri, ignore = "десять тысяч задач под Miri идут долго")]
fn b_ten_thousand_timers() {
    use std::cell::Cell;
    use std::rc::Rc;
    let rt = sut::b_timers::Runtime::new();
    let done = Rc::new(Cell::new(0u32));
    for i in 0..10_000u64 {
        let (h, done) = (rt.handle(), Rc::clone(&done));
        rt.spawn(async move {
            h.sleep(60_000 + i % 100).await; // «минута» виртуального времени
            done.set(done.get() + 1);
        });
    }
    assert_eq!(rt.run(), 60_099);
    assert_eq!(done.get(), 10_000);
}

#[test]
fn b_cancelled_sleep_does_not_move_the_clock() {
    let rt = sut::b_timers::Runtime::new();
    let h = rt.handle();
    rt.spawn(async move {
        let short = pin!(h.sleep(10));
        let long = pin!(h.sleep(1000));
        let won = Select(short, long).await;
        assert!(matches!(won, Either::Left(())), "короткий сон победил");
        assert_eq!(h.now(), 10);
    }); // долгий сон уронен — его таймер не должен держать часы
    assert_eq!(rt.run(), 10);
}

// --- Задача C: реактор на epoll и эхо-сервер ---

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod c_epoll {
    use super::sut::c_epoll::{Stats, serve, serve_threads};
    use std::io::{Read, Write};
    use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    /// Тайм-ауты клиента: пока реактор не дописан, тест падает, а не виснет.
    const LIMIT: Option<Duration> = Some(Duration::from_secs(20));

    /// Резидентная память процесса, КиБ (строка VmRSS в /proc/self/status).
    fn rss_kib() -> usize {
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        let line = status.lines().find(|l| l.starts_with("VmRSS:")).unwrap();
        line.split_whitespace().nth(1).unwrap().parse().unwrap()
    }

    /// `n` клиентов по `len` байт против сервера на `addr`. Клиент
    /// закрывается, только когда эхо получили все: значит, в этот момент
    /// сервер держит все соединения сразу. Тогда же снимается RSS процесса
    /// (КиБ) — его и возвращаем.
    fn clients(addr: SocketAddr, n: usize, len: usize) -> usize {
        let barrier = Arc::new(Barrier::new(n));
        let rss = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..n)
            .map(|i| {
                let (barrier, rss) = (Arc::clone(&barrier), Arc::clone(&rss));
                thread::spawn(move || {
                    let msg: Vec<u8> = (0..len).map(|j| (i * 7 + j) as u8).collect();
                    let talk = || -> std::io::Result<(TcpStream, Vec<u8>)> {
                        let mut s = TcpStream::connect(addr)?;
                        s.set_read_timeout(LIMIT)?;
                        s.set_write_timeout(LIMIT)?;
                        s.write_all(&msg)?;
                        let mut back = vec![0u8; len];
                        s.read_exact(&mut back)?;
                        Ok((s, back))
                    };
                    let res = talk();
                    // До барьера доходят и при ошибке, чтобы не повесить соседей.
                    if barrier.wait().is_leader() {
                        rss.store(rss_kib(), Ordering::Relaxed);
                    }
                    barrier.wait(); // никто не закрывается до замера
                    let (mut s, back) = res.unwrap_or_else(|e| panic!("клиент {i}: {e}"));
                    assert_eq!(back, msg, "клиент {i}: эхо искажено");
                    s.shutdown(Shutdown::Write).unwrap();
                    assert_eq!(s.read(&mut [0u8; 1]).unwrap(), 0, "сервер закрыл соединение");
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        rss.load(Ordering::Relaxed)
    }

    #[test]
    #[cfg_attr(miri, ignore = "asm и сеть под Miri не исполняются")]
    fn c_one_thread_serves_many_clients() {
        const CLIENTS: usize = 1000;
        const LEN: usize = 3000;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || serve(listener, CLIENTS));
        clients(addr, CLIENTS, LEN);
        let stats = server.join().unwrap().unwrap();
        // peak == CLIENTS: все тысяча соединений жили одновременно — на
        // одном потоке сервера.
        assert_eq!(
            stats,
            Stats { accepted: CLIENTS, bytes: CLIENTS * LEN, errors: 0, peak: CLIENTS }
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "потоки и сеть под Miri не исполняются")]
    fn c_thread_server_gives_same_echo() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || serve_threads(listener, 50));
        clients(addr, 50, 3000);
        let stats = server.join().unwrap().unwrap();
        assert_eq!(stats, Stats { accepted: 50, bytes: 50 * 3000, errors: 0, peak: 50 });
    }

    /// Замер для сравнения в разборе задачи: время и память на 1000
    /// соединений у реактора и у сервера на потоках. Клиенты одинаковые,
    /// поэтому разница RSS — цена серверной стороны.
    #[test]
    #[ignore = "замер: cargo test -p ch22-solutions --release -- --ignored c_compare --nocapture"]
    fn c_compare_reactor_and_threads() {
        const N: usize = 1000;
        const LEN: usize = 3000;
        let mut rows = Vec::new();
        for threads in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let start = Instant::now();
            let server = thread::spawn(move || {
                if threads { serve_threads(listener, N) } else { serve(listener, N) }
            });
            let rss = clients(addr, N, LEN);
            let stats = server.join().unwrap().unwrap();
            assert_eq!((stats.accepted, stats.errors, stats.peak), (N, 0, N));
            rows.push((if threads { "потоки" } else { "реактор" }, start.elapsed(), rss));
        }
        for (name, time, rss) in &rows {
            println!("{name:8} {N} соединений: {:>5} мс, RSS {rss} КиБ", time.as_millis());
        }
        let delta = rows[1].2 as f64 - rows[0].2 as f64;
        println!("разница RSS на соединение: {:.1} КиБ", delta / N as f64);
    }

    #[test]
    #[cfg_attr(miri, ignore = "asm и сеть под Miri не исполняются")]
    fn c_large_transfer_survives_full_buffers() {
        // Читатель клиента начинает с опозданием, и буферы сокетов успевают
        // заполниться. Упрётся ли запись сервера в WouldBlock, решают размеры
        // буферов ядра (sysctl net.ipv4.tcp_rmem и tcp_wmem): здесь проверяем
        // целость данных, а ожидание EPOLLOUT наверняка — следующий тест.
        const LEN: usize = 16 << 20;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || serve(listener, 1));
        let s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(LIMIT).unwrap();
        s.set_write_timeout(LIMIT).unwrap();
        let mut reader = s.try_clone().unwrap();
        let receiver = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(100));
            let mut got = Vec::with_capacity(LEN);
            reader.read_to_end(&mut got).unwrap();
            got
        });
        let msg: Vec<u8> = (0..LEN).map(|j| (j * 31 % 251) as u8).collect();
        (&s).write_all(&msg).unwrap();
        s.shutdown(Shutdown::Write).unwrap();
        let got = receiver.join().unwrap();
        assert!(got == msg, "эхо 16 МиБ искажено");
        let stats = server.join().unwrap().unwrap();
        assert_eq!((stats.accepted, stats.bytes, stats.errors), (1, LEN, 0));
    }

    #[test]
    #[cfg_attr(miri, ignore = "asm и сеть под Miri не исполняются")]
    fn c_writer_waits_for_epollout() {
        use super::sut::c_epoll::{Async, Runtime};
        use std::io::ErrorKind;
        use std::os::unix::net::UnixStream;
        // Буфер локального сокета — сотни килобайт и сам не растёт (sysctl
        // net.core.wmem_default), так что 4 МиБ без читателя не влезут.
        // Читатель молчит, пока запись не упрётся в WouldBlock, — значит,
        // сервер обязан дождаться EPOLLOUT, как бы ни легли потоки.
        const LEN: usize = 4 << 20;
        let (ours, mut theirs) = UnixStream::pair().unwrap();
        ours.set_nonblocking(true).unwrap();
        theirs.set_read_timeout(LIMIT).unwrap();
        let blocked = Arc::new(AtomicUsize::new(0));
        let reader = {
            let blocked = Arc::clone(&blocked);
            thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(20);
                while blocked.load(Ordering::Acquire) == 0 {
                    assert!(Instant::now() < deadline, "запись так и не упёрлась в WouldBlock");
                    thread::sleep(Duration::from_millis(1));
                }
                let mut got = Vec::with_capacity(LEN);
                theirs.read_to_end(&mut got).unwrap();
                got
            })
        };
        let msg: Vec<u8> = (0..LEN).map(|j| (j % 253) as u8).collect();
        let rt = Runtime::new().unwrap();
        {
            let (msg, blocked) = (msg.clone(), Arc::clone(&blocked));
            let ours = Async::new(ours, rt.reactor());
            rt.spawn(async move {
                let mut sent = 0;
                while sent < LEN {
                    sent += ours
                        .write_with(|mut s| {
                            let r = s.write(&msg[sent..]);
                            if r.as_ref().is_err_and(|e| e.kind() == ErrorKind::WouldBlock) {
                                blocked.fetch_add(1, Ordering::Release);
                            }
                            r
                        })
                        .await
                        .unwrap();
                }
            }); // ours уронен — читатель увидит конец потока
        }
        rt.run().unwrap();
        assert!(
            blocked.load(Ordering::Acquire) > 0,
            "запись ни разу не ждала — буфер больше 4 МиБ?"
        );
        assert!(reader.join().unwrap() == msg, "данные искажены");
    }
}

// --- Задача D1: исполнитель без кучи на статических задачах ---

mod d1_static {
    use super::sut::d1_static::{Arena, ByteQueue, Executor, Report, Signal};
    use std::future::Future;
    use std::mem::MaybeUninit;
    use std::pin::Pin;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context, Poll};

    /// Статический буфер арены, выдаваемый один раз, — то, что на Cortex-M
    /// делает `cortex_m::singleton!`. (Утечка кучи через `Box::leak` тоже
    /// дала бы `'static`, но Miri справедливо назвал бы её утечкой.)
    struct StaticBuf<const N: usize> {
        taken: AtomicBool,
        mem: std::cell::UnsafeCell<[MaybeUninit<u8>; N]>,
    }

    // SAFETY: mem выдаётся наружу не больше одного раза (флаг taken).
    unsafe impl<const N: usize> Sync for StaticBuf<N> {}

    impl<const N: usize> StaticBuf<N> {
        const fn new() -> Self {
            StaticBuf {
                taken: AtomicBool::new(false),
                mem: std::cell::UnsafeCell::new([MaybeUninit::uninit(); N]),
            }
        }
    }

    fn arena<const N: usize>(buf: &'static StaticBuf<N>) -> Arena {
        assert!(!buf.taken.swap(true, Ordering::AcqRel), "буфер уже выдан");
        // SAFETY: флаг гарантирует, что это единственная ссылка на mem.
        Arena::new(unsafe { &mut *buf.mem.get() })
    }

    #[test]
    fn d1_arena_aligns_and_refuses_when_full() {
        #[repr(align(16))]
        struct Aligned([u8; 40]);
        impl Future for Aligned {
            type Output = ();
            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
                assert_eq!(self.0[39], 7);
                Poll::Ready(())
            }
        }
        static BUF: StaticBuf<128> = StaticBuf::new();
        let mut a = arena(&BUF);
        a.alloc(async {}).expect("1 байт влезает");
        let before = a.remaining();
        let mut big = a.alloc(Aligned([7; 40])).expect("48 байт влезают");
        let addr = &*big as *const dyn Future<Output = ()> as *const u8 as usize;
        assert_eq!(addr % 16, 0, "future выровнена под свой тип");
        assert!(before - a.remaining() >= size_of::<Aligned>(), "занято не меньше размера");
        // Пока влезает — Some; потом None, и неудача арену не трогает.
        while a.alloc(Aligned([0; 40])).is_some() {}
        let left = a.remaining();
        assert!(a.alloc(Aligned([0; 40])).is_none());
        assert_eq!(a.remaining(), left);
        assert!(left < size_of::<Aligned>() + align_of::<Aligned>(), "остаток мал");
        // Размещённые задачи живые: их можно опрашивать.
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert!(big.as_mut().poll(&mut cx).is_ready());
    }

    #[test]
    fn d1_only_woken_task_is_polled() {
        static FLAGS: [AtomicBool; 2] = [const { AtomicBool::new(false) }; 2];
        static TICKS: Signal = Signal::new(&FLAGS[0]);
        static RX: ByteQueue<8> = ByteQueue::new(&FLAGS[1]);
        static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

        static BUF: StaticBuf<512> = StaticBuf::new();
        let mut a = arena(&BUF);
        let ticker = a
            .alloc(async {
                let mut seen = 0;
                for _ in 0..3 {
                    let t = TICKS.wait(&mut seen).await;
                    LOG.lock().unwrap().push(format!("тик {t}"));
                }
            })
            .unwrap();
        let line = a
            .alloc(async {
                let mut got = Vec::new();
                loop {
                    match RX.pop().await {
                        b'\n' => break,
                        b => got.push(b),
                    }
                }
                LOG.lock().unwrap().push(format!("uart: {}", String::from_utf8(got).unwrap()));
            })
            .unwrap();
        // «Прерывания»: каждый уход в сон будит ровно одно событие.
        let mut events = b"TpiTng\nT".iter();
        let report = Executor::new(&FLAGS, [ticker, line]).run(|_| match events.next() {
            Some(b'T') => TICKS.raise(),
            Some(&b) => RX.push(b),
            None => panic!("исполнитель уснул, хотя событий больше нет"),
        });
        assert_eq!(*LOG.lock().unwrap(), ["тик 1", "тик 2", "uart: ping", "тик 3"]);
        // Первый опрос каждой + по опросу на каждое её событие: соседку
        // чужие прерывания не будят.
        assert_eq!(report, Report { polls: [4, 6], idles: 8 });
        assert_eq!(RX.lost(), 0);
    }

    #[test]
    fn d1_wake_of_finished_task_is_ignored() {
        static FLAGS: [AtomicBool; 2] = [const { AtomicBool::new(false) }; 2];
        static TICKS: Signal = Signal::new(&FLAGS[1]);
        static BUF: StaticBuf<256> = StaticBuf::new();
        let mut a = arena(&BUF);
        let quick = a.alloc(async {}).unwrap();
        let waiter = a
            .alloc(async {
                TICKS.wait(&mut 0).await;
            })
            .unwrap();
        let mut idles = 0;
        let report = Executor::new(&FLAGS, [quick, waiter]).run(|flags| {
            idles += 1;
            match idles {
                1 => flags[0].store(true, Ordering::Release), // лишний будильник
                2 => TICKS.raise(),
                _ => panic!("исполнитель крутится без дела"),
            }
        });
        assert_eq!(report, Report { polls: [1, 2], idles: 2 });
    }

    #[test]
    fn d1_byte_queue_drops_and_counts_overflow() {
        static FLAG: AtomicBool = AtomicBool::new(false);
        static Q: ByteQueue<4> = ByteQueue::new(&FLAG);
        for b in 1..=6 {
            Q.push(b); // потребитель молчит: два байта лишние
        }
        assert_eq!(Q.lost(), 2);
        assert!(FLAG.load(Ordering::Acquire), "push будит задачу");
        let got: Vec<u8> = (0..4).map(|_| super::poll_to_end(Q.pop()).0).collect();
        assert_eq!(got, [1, 2, 3, 4]);
    }
}

// --- Задача D2: отменобезопасное чтение кадров ---

/// Кадры для D2: длина, затем тело; есть пустой и длинный.
fn d2_frames() -> Vec<Vec<u8>> {
    vec![b"hello".to_vec(), Vec::new(), b"world!!".to_vec(), (0..200).collect(), b"end".to_vec()]
}

fn d2_encode(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for f in frames {
        out.push(f.len() as u8);
        out.extend_from_slice(f);
    }
    out
}

#[test]
fn d2_reader_without_cancellation() {
    let frames = d2_frames();
    let mut src = sut::d2_cancel::Source::new(&d2_encode(&frames));
    let mut reader = sut::d2_cancel::FrameReader::new();
    let mut got = Vec::new();
    while let (Some(f), _) = poll_to_end(reader.read_frame(&mut src)) {
        got.push(f);
    }
    assert_eq!(got, frames);
}

#[test]
fn d2_reader_survives_repeated_timeouts() {
    let frames = d2_frames();
    let mut src = sut::d2_cancel::Source::new(&d2_encode(&frames));
    let mut reader = sut::d2_cancel::FrameReader::new();
    let (mut got, mut cancelled) = (Vec::new(), 0);
    for attempt in 0.. {
        // Каждое чтение — наперегонки с тайм-аутом в 1–4 опроса.
        let read = pin!(reader.read_frame(&mut src));
        let timeout = pin!(Timeout(1 + attempt % 4));
        match poll_to_end(Select(read, timeout)).0 {
            Either::Left(Some(f)) => got.push(f),
            Either::Left(None) => break,
            Either::Right(()) => cancelled += 1, // уронили посреди кадра
        }
    }
    assert!(cancelled > 50, "отмен было мало: {cancelled}");
    assert_eq!(got, frames, "ни один байт не потерян");
}

#[test]
fn d2_naive_reader_loses_sync_after_timeout() {
    let frames = d2_frames();
    let mut src = sut::d2_cancel::Source::new(&d2_encode(&frames));
    {
        let read = pin!(sut::d2_cancel::read_frame_naive(&mut src));
        let timeout = pin!(Timeout(2));
        let res = poll_to_end(Select(read, timeout)).0;
        assert!(matches!(res, Either::Right(())), "тайм-аут победил");
    } // длина 5 и «h» ушли вместе с автоматом
    assert_eq!(src.consumed(), 2);
    let (next, _) = poll_to_end(sut::d2_cancel::read_frame_naive(&mut src));
    let next = next.expect("байтов в источнике ещё много");
    assert_eq!(next.len(), b'e' as usize, "длиной кадра прочитана буква «e»");
    assert!(!frames.contains(&next));
}
