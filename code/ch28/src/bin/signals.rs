//! Раздел 28.4: опыты с сигналами. Каждый опыт меняет обработчики и маски
//! всего процесса, поэтому это отдельная программа, а не тест.
//!
//! - `signals flag` — SIGTERM поднимает флаг, цикл работы его видит.
//! - `signals eintr` — read, прерванный сигналом, с SA_RESTART и без, и
//!   что с этим делает std.
//! - `signals signalfd` — сигнал как чтение дескриптора.
//! - `signals println-in-handler` — `println!` в обработчике, пока
//!   программа сама пишет в stdout.
//! - `signals sigpipe` — печать в закрытый канал.

use ch28::signals::{install_stop_flag, last_signal, signal_fd};
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

fn flag() {
    let stop = install_stop_flag().unwrap();
    println!("pid {}", std::process::id());
    io::stdout().flush().unwrap();
    let mut steps = 0u64;
    while !stop.load(Ordering::Relaxed) {
        steps += 1;
        thread::sleep(Duration::from_millis(1));
    }
    let name = Signal::try_from(last_signal() as i32).map_or("?", |s| s.as_str());
    // Сигнал может прийти и до первого шага: число шагов — от нуля.
    println!("остановлен по {name}, шагов сделано: {steps}");
}

// ANCHOR: alarm
extern "C" fn on_alarm(_: libc::c_int) {}

/// Пустой обработчик SIGALRM; `restart` — флаг SA_RESTART.
fn on_sigalrm(restart: bool) {
    let flags = if restart { SaFlags::SA_RESTART } else { SaFlags::empty() };
    let action = SigAction::new(SigHandler::Handler(on_alarm), flags, SigSet::empty());
    // SAFETY: обработчик пустой.
    unsafe { sigaction(Signal::SIGALRM, &action).unwrap() };
}

/// SIGALRM через `ms` миллисекунд.
fn alarm_in(ms: u64) {
    let t = libc::itimerval {
        it_interval: libc::timeval { tv_sec: 0, tv_usec: 0 },
        it_value: libc::timeval { tv_sec: 0, tv_usec: (ms * 1000) as libc::suseconds_t },
    };
    // SAFETY: указатель на живую структуру, старое значение не нужно.
    assert_eq!(unsafe { libc::setitimer(libc::ITIMER_REAL, &t, std::ptr::null_mut()) }, 0);
}

/// Канал, в который другой поток запишет байт через `ms` миллисекунд.
/// Поток блокирует SIGALRM: сигнал процессу должен достаться главному
/// потоку, который ждёт в read.
fn pipe_with_late_byte(ms: u64) -> File {
    let (r, w) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC).unwrap();
    thread::spawn(move || {
        SigSet::from(Signal::SIGALRM).thread_block().unwrap();
        thread::sleep(Duration::from_millis(ms));
        rustix::io::write(&w, b"x").ok();
    });
    File::from(r)
}
// ANCHOR_END: alarm

// ANCHOR: eintr
/// Время с `t`, округлённое вниз до 100 мс.
fn ms(t: Instant) -> u128 {
    t.elapsed().as_millis() / 100 * 100
}

fn eintr() {
    let mut buf = [0u8; 1];
    let errno = |e: rustix::io::Errno| nix::errno::Errno::from_raw(e.raw_os_error());

    on_sigalrm(false);
    let pipe = pipe_with_late_byte(400);
    let t = Instant::now();
    alarm_in(200);
    let r = rustix::io::read(&pipe, &mut buf).map_err(errno);
    println!("без SA_RESTART: read {r:?} через {} мс", ms(t));

    on_sigalrm(true);
    let pipe = pipe_with_late_byte(400);
    let t = Instant::now();
    alarm_in(200);
    let r = rustix::io::read(&pipe, &mut buf).map_err(errno);
    println!("с SA_RESTART:   read {r:?} через {} мс", ms(t));

    on_sigalrm(false);
    let mut pipe = pipe_with_late_byte(400);
    let t = Instant::now();
    alarm_in(200);
    let r = pipe.read(&mut buf).map_err(|e| e.kind());
    println!("std read:       {r:?} через {} мс", ms(t));

    let mut pipe = pipe_with_late_byte(400);
    let t = Instant::now();
    alarm_in(200);
    let r = pipe.read_exact(&mut buf).map_err(|e| e.kind());
    println!("std read_exact: {r:?} через {} мс", ms(t));
}
// ANCHOR_END: eintr

// ANCHOR: signalfd
fn signalfd() {
    let sfd = signal_fd(&[Signal::SIGUSR1]).unwrap();
    let me = std::process::id().to_string();
    let killer = std::process::Command::new("kill").args(["-USR1", &me]).spawn().unwrap();
    let killer_pid = killer.id();
    let info = sfd.read_signal().unwrap().expect("блокирующее чтение");
    let name = Signal::try_from(info.ssi_signo as i32).map_or("?", |s| s.as_str());
    println!("{name} от pid {}: это kill — {}", info.ssi_pid, info.ssi_pid == killer_pid);
    killer.wait_with_output().unwrap();
}
// ANCHOR_END: signalfd

// ANCHOR: println
extern "C" fn on_alarm_print(_: libc::c_int) {
    println!("сигнал!"); // не async-signal-safe
}

fn println_in_handler() {
    let action =
        SigAction::new(SigHandler::Handler(on_alarm_print), SaFlags::empty(), SigSet::empty());
    // SAFETY: нарушено намеренно — обработчик печатает.
    unsafe { sigaction(Signal::SIGALRM, &action).unwrap() };
    alarm_in(200);
    // Канал никто не читает: write в нём заснёт, держа буфер stdout.
    io::stdout().write_all(&vec![b'.'; 1 << 20]).unwrap();
}
// ANCHOR_END: println

fn sigpipe() {
    for i in 0.. {
        println!("строка {i}");
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("flag") => flag(),
        Some("eintr") => eintr(),
        Some("signalfd") => signalfd(),
        Some("println-in-handler") => println_in_handler(),
        Some("sigpipe") => sigpipe(),
        _ => {
            eprintln!(
                "использование: signals flag | eintr | signalfd | println-in-handler | sigpipe"
            );
            std::process::exit(2);
        }
    }
}
