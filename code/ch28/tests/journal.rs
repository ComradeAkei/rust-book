//! Раздел 28.8: демон журнала — отдельный процесс, которому тест пишет в
//! stdin, шлёт сигналы и которого убивает SIGKILL. Журнал после этого
//! читается через `read_committed` и разбирается `Frames` из главы 13.

use ch28::journal::read_committed;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use telemetry_frame::Measurement;
use telemetry_frame::stream::{Frames, stats};

fn journal_path(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch28-{name}.journal"));
    std::fs::remove_file(&p).ok();
    p
}

// ANCHOR: daemon
struct Daemon {
    child: Child,
    stdin: Option<ChildStdin>,
    out: BufReader<ChildStdout>,
}

impl Daemon {
    /// Запустить демон и дождаться строки «журнал открыт»: после неё
    /// маска сигналов уже стоит и сигнал не убьёт процесс.
    fn start(path: &Path) -> (Daemon, String) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_journal"))
            .arg("run")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut d = Daemon { child, stdin, out };
        let first = d.line();
        (d, first)
    }

    fn line(&mut self) -> String {
        let mut s = String::new();
        self.out.read_line(&mut s).unwrap();
        s.trim_end().to_owned()
    }

    /// Строка в stdin демона и его ответ.
    fn send(&mut self, line: &str) -> String {
        writeln!(self.stdin.as_mut().unwrap(), "{line}").unwrap();
        self.line()
    }

    fn signal(&self, sig: Signal) {
        kill(Pid::from_raw(self.child.id() as i32), sig).unwrap();
    }
}
// ANCHOR_END: daemon

// ANCHOR: sigkill
#[test]
fn sigkill_keeps_committed_frames() {
    let path = journal_path("sigkill");
    let (mut d, first) = Daemon::start(&path);
    assert_eq!(first, "журнал открыт: кадров 0, занято 0 из 65536 байт");
    assert_eq!(d.send("1 2000"), "кадр 0: датчик 1 = 2000");
    assert_eq!(d.send("2 -500"), "кадр 1: датчик 2 = -500");
    assert_eq!(d.send("3 7"), "кадр 2: датчик 3 = 7");
    d.signal(Signal::SIGKILL);
    let status = d.child.wait().unwrap();
    assert_eq!(status.signal(), Some(libc::SIGKILL));

    // msync не было, процесса нет — кадры в страничном кеше.
    let bytes = read_committed(&path).unwrap();
    let s = stats(&bytes);
    assert_eq!((s.frames, s.errors, s.junk), ([3, 0, 0], 0, 0));
    let values: Vec<(u16, i32)> = Frames::until_eof(&bytes)
        .map(|f| Measurement::from_bytes(f.unwrap().1.payload).unwrap())
        .map(|m| (m.sensor.get(), m.value))
        .collect();
    assert_eq!(values, [(1, 2000), (2, -500), (3, 7)]);
}
// ANCHOR_END: sigkill

#[test]
fn sighup_reports_sigterm_syncs_and_exits() {
    let path = journal_path("signals");
    let (mut d, _) = Daemon::start(&path);
    d.send("1 10");
    d.send("1 11");
    // Строка не по формату — сообщение в stderr, в stdout ответа нет:
    // следующий ответ — уже на следующую строку.
    writeln!(d.stdin.as_mut().unwrap(), "не по формату").unwrap();
    assert_eq!(d.send("1 12"), "кадр 2: датчик 1 = 12");
    d.signal(Signal::SIGHUP);
    assert_eq!(d.line(), "SIGHUP: кадров 3, измерений 3, байт 54");
    d.signal(Signal::SIGTERM);
    assert_eq!(d.line(), "SIGTERM: msync, кадров 3");
    assert_eq!(d.child.wait().unwrap().code(), Some(0));
    let mut err = String::new();
    d.child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    assert_eq!(err, "пропущено: ожидалось «датчик значение»: \"не по формату\"\n");
}

#[test]
fn restart_continues_the_journal() {
    let path = journal_path("restart");
    let (mut d, _) = Daemon::start(&path);
    d.send("5 1");
    drop(d.stdin.take()); // конец ввода
    assert_eq!(d.line(), "конец ввода: msync, кадров 1");
    assert!(d.child.wait().unwrap().success());

    let (mut d, first) = Daemon::start(&path);
    assert_eq!(first, "журнал открыт: кадров 1, занято 18 из 65536 байт");
    assert_eq!(d.send("5 2"), "кадр 1: датчик 5 = 2");
    d.signal(Signal::SIGINT);
    assert_eq!(d.line(), "SIGINT: msync, кадров 2");
    assert!(d.child.wait().unwrap().success());
}

#[test]
fn stop_and_continue_do_not_break_the_loop() {
    // Ctrl-Z и `fg`: остановка и SIGCONT прерывают epoll_wait с EINTR и
    // без обработчиков (signal(7)). Паузы — чтобы демон успел дойти до
    // epoll_wait и успел остановиться.
    let path = journal_path("stop");
    let (mut d, _) = Daemon::start(&path);
    assert_eq!(d.send("4 1"), "кадр 0: датчик 4 = 1");
    for _ in 0..3 {
        sleep(Duration::from_millis(50));
        d.signal(Signal::SIGSTOP);
        sleep(Duration::from_millis(50));
        d.signal(Signal::SIGCONT);
    }
    assert_eq!(d.send("4 2"), "кадр 1: датчик 4 = 2");
    drop(d.stdin.take());
    assert_eq!(d.line(), "конец ввода: msync, кадров 2");
    assert!(d.child.wait().unwrap().success());
}

#[test]
fn second_daemon_on_same_journal_is_refused() {
    let path = journal_path("lock");
    let (mut first, _) = Daemon::start(&path);
    let second = Command::new(env!("CARGO_BIN_EXE_journal"))
        .arg("run")
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(second.status.code(), Some(1));
    let err = String::from_utf8(second.stderr).unwrap();
    assert!(err.contains("Resource temporarily unavailable"), "{err}");
    first.signal(Signal::SIGTERM);
    assert_eq!(first.line(), "SIGTERM: msync, кадров 0");
    first.child.wait().unwrap();
}
