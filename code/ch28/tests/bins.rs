//! Опыты глав 28.2–28.7 — отдельные программы из src/bin: они меняют
//! обработчики сигналов, делают fork и считают страничные сбои всего
//! потока. Тест запускает каждую и сверяет вывод с текстом главы.
//! Времена и счётчики, которые зависят от машины, проверяются с запасом.

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn run(exe: &str, args: &[&str]) -> Output {
    Command::new(exe).args(args).output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

/// Число после `prefix` в строке вывода.
fn number_after(text: &str, prefix: &str) -> f64 {
    let line =
        text.lines().find(|l| l.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix}: {text}"));
    let rest = line[prefix.len()..].trim_start();
    let end =
        rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(rest.len());
    rest[..end].parse().unwrap_or_else(|_| panic!("{line}"))
}

#[test]
fn layers_print_and_report_one_errno() {
    let o = run(env!("CARGO_BIN_EXE_layers"), &[]);
    assert!(o.status.success());
    assert_eq!(
        stdout(&o),
        "std\nlibc\nrustix\nnix\n\
         std    8 байт: No such file or directory (os error 2)\n\
         libc   4 байт: No such file or directory (os error 2)\n\
         rustix 2 байт: No such file or directory (os error 2)\n\
         nix    4 байт: ENOENT: No such file or directory\n"
    );
}

#[test]
fn double_close_is_caught_in_debug_builds() {
    let o = run(env!("CARGO_BIN_EXE_fds"), &["double-close"]);
    let err = stderr(&o);
    assert!(err.starts_with("первый владелец закрыл "), "{err}");
    if cfg!(debug_assertions) {
        assert!(err.contains("IO Safety violation: owned file descriptor already closed"), "{err}");
        assert_eq!(o.status.signal(), Some(libc::SIGABRT));
    } else {
        assert!(err.contains("второй владелец закрыл"), "{err}");
    }
}

#[test]
fn fork_under_held_mutex_hangs_the_child() {
    let o = run(env!("CARGO_BIN_EXE_procs"), &["fork-mutex"]);
    let out = stdout(&o);
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with("ребёнок ")
            && lines[0].ends_with(": потоков 1, состояние S (sleeping)"),
        "{out}"
    );
    assert_eq!(
        lines[1..],
        [
            "родитель: поток отпустил мьютекс, в таблице 1",
            "ребёнок всё ещё ждёт мьютекс: отпустить его некому"
        ]
    );
}

#[test]
fn parent_death_kills_the_tied_child() {
    let o = run(env!("CARGO_BIN_EXE_procs"), &["tied"]);
    assert!(o.status.success());
    let pid: u32 = stdout(&o).trim().parse().unwrap();
    // Ребёнка убивает ядро; кто его пожнёт, зависит от системы: init
    // контейнера может оставить зомби.
    let start = Instant::now();
    loop {
        match ch28::process::proc_status(pid, "State") {
            None => break,
            Some(s) if s.starts_with('Z') => break,
            Some(s) => assert!(start.elapsed() < Duration::from_secs(5), "ребёнок {pid} жив: {s}"),
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn child_inherits_the_signal_mask() {
    let o = run(env!("CARGO_BIN_EXE_procs"), &["mask"]);
    // SIGTERM — 15-й сигнал, бит 14: 0x4000.
    assert_eq!(
        stdout(&o),
        "обычный Command:     SigBlk:\t0000000000004000\n\
         pre_exec с маской []: SigBlk:\t0000000000000000\n"
    );
}

#[test]
fn stop_flag_sees_sigterm() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_signals"))
        .arg("flag")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    out.read_line(&mut line).unwrap();
    assert_eq!(line.trim(), format!("pid {}", child.id()));
    kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM).unwrap();
    line.clear();
    out.read_to_string(&mut line).unwrap();
    assert!(line.starts_with("остановлен по SIGTERM, шагов сделано: "), "{line}");
    assert!(child.wait().unwrap().success());
}

#[test]
fn eintr_with_and_without_sa_restart() {
    let o = run(env!("CARGO_BIN_EXE_signals"), &["eintr"]);
    let out = stdout(&o);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 4, "{out}");
    // Сигнал — через 200 мс, байт в канале — через 400 мс.
    let ms = |l: &str| {
        l.rsplit(" через ").next().unwrap().trim_end_matches(" мс").parse::<u32>().unwrap()
    };
    assert!(lines[0].starts_with("без SA_RESTART: read Err(EINTR) через "), "{out}");
    assert!((200..400).contains(&ms(lines[0])), "{out}");
    assert!(lines[1].starts_with("с SA_RESTART:   read Ok(1) через "), "{out}");
    assert!(ms(lines[1]) >= 400, "{out}");
    assert!(lines[2].starts_with("std read:       Err(Interrupted) через "), "{out}");
    assert!((200..400).contains(&ms(lines[2])), "{out}");
    assert!(lines[3].starts_with("std read_exact: Ok(()) через "), "{out}");
    assert!(ms(lines[3]) >= 400, "{out}");
}

#[test]
fn signalfd_reports_the_sender() {
    let o = run(env!("CARGO_BIN_EXE_signals"), &["signalfd"]);
    let out = stdout(&o);
    assert!(out.starts_with("SIGUSR1 от pid ") && out.ends_with(": это kill — true\n"), "{out}");
}

#[test]
fn println_in_handler_aborts() {
    // stdout — канал, который никто не читает: write в программе заснёт,
    // держа буфер stdout, и обработчик SIGALRM придёт к занятому RefCell.
    let mut child = Command::new(env!("CARGO_BIN_EXE_signals"))
        .arg("println-in-handler")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    let status = child.wait().unwrap();
    assert!(err.contains("RefCell already borrowed"), "{err}");
    assert!(err.contains("panic in a function that cannot unwind"), "{err}");
    assert_eq!(status.signal(), Some(libc::SIGABRT));
}

#[test]
fn sigpipe_is_ignored_and_print_panics() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_signals"))
        .arg("sigpipe")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut first).unwrap();
    assert_eq!(first, "строка 0\n");
    // Читатель закрыт: следующий write получит EPIPE, а не SIGPIPE.
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    assert!(err.contains("failed printing to stdout: Broken pipe (os error 32)"), "{err}");
    assert_eq!(child.wait().unwrap().code(), Some(101));
}

#[test]
fn page_faults_by_kind() {
    let o = run(env!("CARGO_BIN_EXE_mm"), &["faults"]);
    assert!(o.status.success(), "{}", stderr(&o));
    let out = stdout(&o);
    let small = number_after(&out, "анонимная, 4 КиБ:");
    let huge = number_after(&out, "анонимная, THP:");
    let file = number_after(&out, "файл, чтение:");
    let populated = number_after(&out, "файл, MAP_POPULATE:");
    // По сбою на страницу: 16384 страницы, немного сверху — стек и прочее.
    assert!((16384.0..16384.0 + 256.0).contains(&small), "{out}");
    let thp =
        std::fs::read_to_string("/sys/kernel/mm/transparent_hugepage/enabled").unwrap_or_default();
    if !thp.contains("[never]") {
        // Огромная страница — 2 МиБ: 32 сбоя, если ядро нашло свободные
        // 2 МиБ; иначе часть памяти останется на 4 КиБ.
        assert!(huge < small / 4.0, "{out}");
    }
    // Чтение файла: ядро отображает сразу соседние страницы из кэша
    // (fault-around, большие фолианты) — сбоев заметно меньше страниц.
    assert!(file < small / 4.0, "{out}");
    assert!(populated < 64.0, "{out}");
}

#[test]
fn truncated_mapping_is_sigbus() {
    let o = run(env!("CARGO_BIN_EXE_mm"), &["sigbus"]);
    assert_eq!(stdout(&o), "до: m[4096] = 1\nпосле укорачивания: m[4096] = …\n");
    assert_eq!(o.status.signal(), Some(libc::SIGBUS));
}

#[test]
fn memfd_shared_between_processes() {
    let o = run(env!("CARGO_BIN_EXE_mm"), &["shared"]);
    assert_eq!(
        stdout(&o),
        "ребёнок видит mem[0] = 41, пишет 42\nродитель после ребёнка: mem[0] = 42\n"
    );
}

#[test]
fn proc_maps_of_own_process() {
    let o = run(env!("CARGO_BIN_EXE_procfs"), &["maps"]);
    let out = stdout(&o);
    let line = |what: &str| {
        out.lines().find(|l| l.starts_with(what)).unwrap_or_else(|| panic!("{what}: {out}"))
    };
    assert!(line("код main   r-xp").ends_with("КиБ  procfs"), "{out}");
    assert!(line("Box        rw-p").ends_with("[heap]"), "{out}");
    let journal = line("журнал     rw-p");
    assert!(journal.ends_with("(без имени)") || journal.ends_with("[anon:ch28-journal]"), "{out}");
    assert!(line("локальная  rw-p").ends_with("[stack]"), "{out}");
    assert!(out.lines().any(|l| l.ends_with("[vdso]") && l.contains("r-xp")), "{out}");
}

#[test]
fn rss_follows_touched_pages() {
    let o = run(env!("CARGO_BIN_EXE_procfs"), &["rss"]);
    let out = stdout(&o);
    let start = number_after(&out, "в начале:            VmRSS");
    let mapped = number_after(&out, "после mmap 64 МиБ:   VmRSS");
    let touched = number_after(&out, "после касания:       VmRSS");
    let unmapped = number_after(&out, "после munmap:        VmRSS");
    assert!(mapped - start < 1.0, "{out}");
    assert!(touched - mapped > 60.0, "{out}");
    assert!(touched - unmapped > 60.0, "{out}");
}

#[test]
fn proc_files_have_no_size() {
    let o = run(env!("CARGO_BIN_EXE_procfs"), &["sizes"]);
    let out = stdout(&o);
    for l in out.lines().filter(|l| l.starts_with("/proc/")) {
        assert!(l.contains("stat:     0, прочитано: "), "{out}");
        let read: u32 = l.rsplit(' ').next().unwrap().parse().unwrap();
        assert!(read > 100, "{out}");
    }
    let cpus = run(env!("CARGO_BIN_EXE_procfs"), &["cpus"]);
    let n = number_after(&stdout(&cpus), "available_parallelism:");
    assert!(n >= 1.0);
}

#[test]
fn vdso_is_cheaper_than_a_syscall() {
    let o = run(env!("CARGO_BIN_EXE_syscost"), &[]);
    let out = stdout(&o);
    assert!(out.ends_with("vDSO быстрее вызова ядра: true\n"), "{out}");
}
