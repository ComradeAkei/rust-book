//! Раздел 30.3: счётчики perf_event_open и профиль perf record; раздел
//! 30.9: профиль приёма кадров станции.
//!
//! Аппаратные счётчики есть не везде: в виртуальной машине без PMU их
//! открытие даёт ENOENT, и тест это принимает. Программные события
//! считает ядро, они есть всегда, если `kernel.perf_event_paranoid`
//! пускает пользователя; при отказе (EACCES) опыт пропускается, а с
//! `CH30_REQUIRE_PERF_EVENTS` в окружении — падает. Опыты с perf
//! пропускаются без perf и при том же отказе; `CH30_REQUIRE_PERF` делает
//! пропуск без perf ошибкой.

mod common;

use ch30::perf_event::{Counter, Event, touch_fresh_pages};
use ch30::pitfalls::sum_squares;
use std::collections::HashMap;
use std::hint::black_box;
use std::io;
use std::process::Command;

fn open(event: Event) -> Option<Counter> {
    match Counter::new(event) {
        Ok(c) => Some(c),
        Err(e)
            if e.raw_os_error() == Some(libc::EACCES)
                && !common::required("CH30_REQUIRE_PERF_EVENTS") =>
        {
            eprintln!("perf_event_open: {e}; проверьте kernel.perf_event_paranoid (≤ 2)");
            None
        }
        Err(e) => panic!("{}: {e}", event.name()),
    }
}

#[test]
fn page_faults_one_per_fresh_page() {
    let Some(c) = open(Event::PageFaults) else { return };
    for pages in [1, 100, 1000] {
        let (_, n) = c.count(|| touch_fresh_pages(pages).unwrap()).unwrap();
        // Сверху — запас на сбои вне отображения: стек, буфер вывода.
        assert!((pages as u64..pages as u64 + 8).contains(&n), "{pages} страниц: {n} сбоев");
    }
}

#[test]
fn task_clock_counts_nanoseconds() {
    let Some(c) = open(Event::TaskClock) else { return };
    let (_, ns) = c.count(|| std::thread::sleep(std::time::Duration::from_millis(20))).unwrap();
    // Сон — не работа: task-clock считает только время на процессоре.
    assert!(ns < 5_000_000, "{ns}");
    let v: Vec<u64> = (0..1_000_000).collect();
    let (_, ns) = c.count(|| black_box(sum_squares(black_box(&v)))).unwrap();
    assert!(ns > 10_000, "{ns}");
}

#[test]
fn observed_hardware_counters_or_enoent() {
    // Без PMU (виртуальная машина) — ENOENT; другие ошибки, которые
    // встречаются у perf без аппаратной поддержки: EOPNOTSUPP и ENODEV.
    for event in Event::ALL.into_iter().filter(|e| e.is_hardware()) {
        match Counter::new(event) {
            Ok(c) => {
                let v: Vec<u64> = (0..100_000).collect();
                let (_, n) = c.count(|| black_box(sum_squares(black_box(&v)))).unwrap();
                eprintln!("{}: {n}", event.name());
                if event == Event::Instructions {
                    assert!(n > 100_000 / 4, "{n}");
                }
            }
            Err(e) => {
                let code = e.raw_os_error().unwrap();
                assert!(
                    [libc::ENOENT, libc::EOPNOTSUPP, libc::ENODEV, libc::EACCES].contains(&code),
                    "{}: {e}",
                    event.name()
                );
                eprintln!("{}: {e}", event.name());
            }
        }
    }
}

#[test]
fn counters_program_matches_text() {
    let out = Command::new(env!("CARGO_BIN_EXE_counters")).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let names: Vec<&str> = text.lines().map(|l| l.split(':').next().unwrap().trim()).collect();
    assert_eq!(names, Event::ALL.map(Event::name));
    if Counter::new(Event::PageFaults).is_ok() {
        let line = text.lines().find(|l| l.contains("page-faults")).unwrap();
        let n: u64 = line.rsplit(' ').next().unwrap().parse().unwrap();
        assert!((1000..1008).contains(&n), "{line}");
    }
}

/// perf, если он есть и ядро пускает к `perf_event_open`: perf считает
/// через тот же вызов, и при `kernel.perf_event_paranoid` выше 2 опыт
/// пропускается, как у счётчиков выше.
fn perf() -> Option<std::path::PathBuf> {
    let p = common::tool("PERF", "perf");
    assert!(p.is_some() || !common::required("CH30_REQUIRE_PERF"), "нужен perf");
    p.filter(|_| open(Event::TaskClock).is_some())
}

/// perf stat в машиночитаемом виде (`-x,`): событие → значение или
/// `<not supported>`. perf и программа идут без огромных страниц
/// ([`common::without_thp`]).
fn perf_stat(perf: &std::path::Path, events: &str, args: &[&str]) -> HashMap<String, String> {
    let out = common::without_thp(&mut Command::new(perf))
        .args(["stat", "-x,", "-e", events, "--"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stderr)
        .unwrap()
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            (f.len() > 2).then(|| (f[2].trim_end_matches(":u").to_owned(), f[0].to_owned()))
        })
        .collect()
}

#[test]
fn perf_stat_page_faults_of_profile() {
    let Some(perf) = perf() else { return };
    let exe = common::release_bin("profile");
    let s = perf_stat(&perf, "task-clock,page-faults,cycles", &[exe.to_str().unwrap(), "16"]);
    // 16 МиБ данных и 16 МиБ отсортированной копии — по сбою на каждую
    // из 2 × 4096 страниц по 4 КиБ, и несколько десятков на запуск
    // программы. Огромных страниц perf_stat программе не даёт.
    let faults: u64 = s["page-faults"].parse().unwrap();
    assert!((8192..8192 + 400).contains(&faults), "{faults}");
    let cycles = &s["cycles"];
    assert!(cycles == "<not supported>" || cycles.parse::<u64>().is_ok(), "{cycles}");
}

/// Стеки `perf script`: кадры от листа к корню, кадр — имя функции без
/// смещения. Имена v0 тест не разбирает: ищет куски декорированного имени.
fn stacks(script: &str) -> Vec<Vec<&str>> {
    script
        .split("\n\n")
        .map(|sample| {
            sample
                .lines()
                .skip(1)
                .filter_map(|l| l.split_whitespace().nth(1))
                .map(|f| f.split('+').next().unwrap())
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn perf_record_finds_sort_hottest() -> io::Result<()> {
    let Some(perf) = perf() else { return Ok(()) };
    let exe = common::release_bin("profile");
    let data = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch30-perf.data");
    let st = Command::new(&perf)
        .args(["record", "-q", "-e", "cpu-clock", "--call-graph", "dwarf", "-o"])
        .arg(&data)
        .arg(&exe)
        .arg("16")
        .stdout(std::process::Stdio::null())
        .status()?;
    assert!(st.success());
    let out = Command::new(&perf).arg("script").arg("-i").arg(&data).output()?;
    assert!(out.status.success());
    let script = String::from_utf8_lossy(&out.stdout);
    let all = stacks(&script);
    // Доля выборок, в стеке которых есть фаза: включающее время, как
    // ширина полосы на flamegraph.
    let share = |part: &str| {
        all.iter().filter(|s| s.iter().any(|f| f.contains(part))).count() as f64 / all.len() as f64
    };
    let (sort, crc, noise, hist) = (
        share("3hot6sorted"),
        share("3hot11crc_bitwise"),
        share("3hot5noise"),
        share("3hot9histogram"),
    );
    eprintln!("sorted {sort:.3}, crc {crc:.3}, noise {noise:.3}, histogram {hist:.3}");
    assert!(sort > 0.5, "{sort}");
    assert!(hist < crc.min(noise), "{hist}");
    Ok(())
}

/// Плоский профиль `perf report --sort symbol`: доля и декорированное имя
/// функции, от самой горячей.
fn flat_profile(
    perf: &std::path::Path,
    exe: &std::path::Path,
    args: &[&str],
    tag: &str,
) -> Vec<(f64, String)> {
    let data =
        std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch30-{tag}.data"));
    let st = Command::new(perf)
        .args(["record", "-q", "-e", "cpu-clock", "-o"])
        .arg(&data)
        .arg(exe)
        .args(args)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(st.success());
    let out = Command::new(perf)
        .args(["report", "--stdio", "--no-children", "--sort", "symbol", "-q", "-i"])
        .arg(&data)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let pct = f.next()?.strip_suffix('%')?.parse().ok()?;
            let name = f.nth(1)?.to_owned();
            Some((pct, name))
        })
        .collect()
}

#[test]
fn perf_record_station_hot_functions() {
    let Some(perf) = perf() else { return };
    let exe = common::release_bin("station");
    // Хранилище проекта: больше половины времени — разбор кадра, и в нём
    // CRC (раздел 30.9).
    let store = flat_profile(&perf, &exe, &["262144", "store"], "station-store");
    eprintln!("{:?}", &store[..3]);
    assert!(store[0].1.ends_with("15telemetry_frame6decode") && store[0].0 > 40.0, "{store:?}");
    // CRC по два байта — отдельная функция и самая горячая.
    let x2 = flat_profile(&perf, &exe, &["262144", "x2"], "station-x2");
    eprintln!("{:?}", &x2[..3]);
    assert!(x2[0].1.ends_with("15crc16_update_x2") && x2[0].0 > 30.0, "{x2:?}");
}
