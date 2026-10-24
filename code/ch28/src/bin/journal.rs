//! Раздел 28.8: демон журнала станции.
//!
//! - `journal run PATH` — строки «датчик значение» со stdin становятся
//!   кадрами измерений в журнале PATH. SIGHUP — сводка, SIGTERM и SIGINT
//!   — msync и выход, конец stdin — то же.
//! - `journal dump PATH` — зафиксированные кадры журнала, прочитанные
//!   через read.

use ch28::journal::{Journal, read_committed};
use ch28::signals::signal_fd;
use nix::sys::signal::Signal;
use rustix::buffer::spare_capacity;
use rustix::event::epoll::{self, CreateFlags, EventData, EventFlags};
use std::num::NonZeroU16;
use std::path::Path;
use std::time::Instant;
use telemetry_frame::stream::{Frames, stats};
use telemetry_frame::{Kind, Measurement};

const CAPACITY: usize = 64 << 10;
const SIGNALS: u64 = 0;
const STDIN: u64 = 1;

// ANCHOR: parse
/// «3 -250» → датчик 3, значение −250.
fn parse_line(line: &str) -> Result<(NonZeroU16, i32), String> {
    let mut it = line.split_whitespace();
    let (Some(sensor), Some(value), None) = (it.next(), it.next(), it.next()) else {
        return Err(format!("ожидалось «датчик значение»: {line:?}"));
    };
    let sensor = sensor.parse().map_err(|e| format!("датчик {sensor:?}: {e}"))?;
    let value = value.parse().map_err(|e| format!("значение {value:?}: {e}"))?;
    Ok((sensor, value))
}
// ANCHOR_END: parse

// ANCHOR: run
fn run(path: &Path) -> i32 {
    // Маска сигналов — до первого потока, иначе поток без маски получит
    // SIGTERM обычным способом и процесс умрёт.
    let signals = signal_fd(&[Signal::SIGTERM, Signal::SIGINT, Signal::SIGHUP]).unwrap();
    // SAFETY: файл журнала — только этого демона; второй экземпляр
    // остановит flock.
    let mut journal = match unsafe { Journal::open(path, CAPACITY) } {
        Ok(j) => j,
        Err(e) => {
            eprintln!("журнал {}: {e}", path.display());
            return 1;
        }
    };
    println!(
        "журнал открыт: кадров {}, занято {} из {} байт",
        journal.frames(),
        journal.committed().len(),
        journal.capacity()
    );

    let ep = epoll::create(CreateFlags::CLOEXEC).unwrap();
    epoll::add(&ep, &signals, EventData::new_u64(SIGNALS), EventFlags::IN).unwrap();
    let stdin = rustix::stdio::stdin();
    epoll::add(&ep, stdin, EventData::new_u64(STDIN), EventFlags::IN).unwrap();

    let start = Instant::now();
    let mut pending = Vec::new();
    let mut events = Vec::with_capacity(2);
    loop {
        events.clear();
        epoll::wait(&ep, spare_capacity(&mut events), None).unwrap();
        for ev in &events {
            match ev.data.u64() {
                SIGNALS => {
                    let info = signals.read_signal().unwrap().unwrap();
                    match Signal::try_from(info.ssi_signo as i32).unwrap() {
                        Signal::SIGHUP => report(&journal),
                        sig => return finish(&journal, sig.as_str()),
                    }
                }
                _ => {
                    let mut buf = [0u8; 4096];
                    let n = rustix::io::read(stdin, &mut buf).unwrap();
                    if n == 0 {
                        return finish(&journal, "конец ввода");
                    }
                    pending.extend_from_slice(&buf[..n]);
                    while let Some(end) = pending.iter().position(|&b| b == b'\n') {
                        let line: Vec<u8> = pending.drain(..=end).collect();
                        let line = String::from_utf8_lossy(&line[..end]);
                        record(&mut journal, &line, start);
                    }
                }
            }
        }
    }
}
// ANCHOR_END: run

// ANCHOR: record
fn record(journal: &mut Journal, line: &str, start: Instant) {
    let (sensor, value) = match parse_line(line) {
        Ok(v) => v,
        Err(e) => return eprintln!("пропущено: {e}"),
    };
    let m = Measurement { sensor, timestamp_ms: start.elapsed().as_millis() as u32, value };
    match journal.push(Kind::Measurement, &m.to_bytes()) {
        Ok(seq) => println!("кадр {seq}: датчик {sensor} = {value}"),
        Err(e) => eprintln!("кадр не записан: {e}"),
    }
}

fn report(journal: &Journal) {
    let s = stats(journal.committed());
    println!(
        "SIGHUP: кадров {}, измерений {}, байт {}",
        journal.frames(),
        s.frames[0],
        journal.committed().len()
    );
}

fn finish(journal: &Journal, why: &str) -> i32 {
    journal.sync().unwrap();
    println!("{why}: msync, кадров {}", journal.frames());
    0
}
// ANCHOR_END: record

// ANCHOR: dump
fn dump(path: &Path) -> i32 {
    let bytes = match read_committed(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("журнал {}: {e}", path.display());
            return 1;
        }
    };
    for item in Frames::until_eof(&bytes) {
        match item {
            Ok((at, f)) => match Measurement::from_bytes(f.payload) {
                Ok(m) => println!(
                    "{at:5}: кадр {:3}, датчик {}, {} мс, {}",
                    f.header.seq, m.sensor, m.timestamp_ms, m.value
                ),
                Err(_) => println!(
                    "{at:5}: кадр {:3}, {:?}, {} байт",
                    f.header.seq,
                    f.header.kind,
                    f.payload.len()
                ),
            },
            Err(e) => println!("ошибка: {e}"),
        }
    }
    let s = stats(&bytes);
    println!(
        "итого: измерений {}, событий {}, ошибок {}, мусора {} байт",
        s.frames[0], s.frames[1], s.errors, s.junk
    );
    0
}
// ANCHOR_END: dump

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match (args.get(1).map(String::as_str), args.get(2)) {
        (Some("run"), Some(p)) => run(Path::new(p)),
        (Some("dump"), Some(p)) => dump(Path::new(p)),
        _ => {
            eprintln!("использование: journal run PATH | dump PATH");
            2
        }
    };
    std::process::exit(code);
}
