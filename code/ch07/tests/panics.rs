//! Разделы 7.5 и 7.7: опыты программы `panics` в отдельных процессах, и
//! раздел 7.1: программа `main_result`.
//! Сверяются строки, которые приводит текст, и способ завершения: обычный
//! выход, код 101 после паники в main или SIGABRT.

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitStatus};

fn run(what: &str) -> (ExitStatus, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_panics"))
        .arg(what)
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap();
    (out.status, String::from_utf8_lossy(&out.stderr).into_owned())
}

fn has(text: &str, lines: &[&str]) {
    for l in lines {
        assert!(text.contains(l), "нет строки «{l}»:\n{text}");
    }
}

/// Строки должны идти в указанном порядке.
fn in_order(text: &str, lines: &[&str]) {
    let mut rest = text;
    for l in lines {
        let i = rest.find(l).unwrap_or_else(|| panic!("нет «{l}» после предыдущих:\n{text}"));
        rest = &rest[i + l.len()..];
    }
}

const ABORT: i32 = 6; // SIGABRT

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn catch_unwind_runs_destructors() {
    let (st, err) = run("catch");
    assert!(st.success());
    in_order(
        &err,
        &[
            "index out of bounds: the len is 3 but the index is 7",
            "drop a",
            "поймали: index out of bounds: the len is 3 but the index is 7",
            "main завершился",
        ],
    );
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn custom_hook() {
    let (st, err) = run("hook");
    assert!(st.success());
    has(&err, &["[станция] паника в code/ch07/src/bin/panics.rs:", ": датчик 3 не отвечает"]);
    has(&err, &["panicked at code/ch07/src/bin/panics.rs:", "снова со стандартным хуком"]);
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn panic_in_drop_during_unwind_aborts() {
    let (st, err) = run("double");
    assert_eq!(st.signal(), Some(ABORT), "{err}");
    in_order(
        &err,
        &[
            "первая паника",
            "drop a",
            "drop Flusher: сброс буфера",
            "сброс не удался",
            "panic in a destructor during cleanup",
            "thread caused non-unwinding panic. aborting.",
        ],
    );
    assert!(!err.contains("main завершился"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn panic_in_drop_alone_unwinds() {
    let (st, err) = run("drop-only");
    assert_eq!(st.code(), Some(101), "{err}");
    has(&err, &["drop Flusher: сброс буфера", "сброс не удался"]);
    assert!(!err.contains("aborting"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn panic_through_extern_c_aborts() {
    let (st, err) = run("ffi-c");
    assert_eq!(st.signal(), Some(ABORT), "{err}");
    in_order(
        &err,
        &[
            "отрицательный отсчёт -1",
            "panic in a function that cannot unwind",
            "thread caused non-unwinding panic. aborting.",
        ],
    );
    assert!(!err.contains("drop a"), "деструкторы не выполнялись");
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn panic_through_c_unwind_is_caught() {
    let (st, err) = run("ffi-c-unwind");
    assert!(st.success());
    in_order(&err, &["отрицательный отсчёт -1", "поймали: true", "drop a", "main завершился"]);
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn track_caller_points_to_caller() {
    let (st, err) = run("track");
    assert_eq!(st.code(), Some(101));
    let src = include_str!("../src/bin/panics.rs");
    let line = src.lines().position(|l| l.contains("reg(std::hint::black_box(40))")).unwrap() + 1;
    has(
        &err,
        &[&format!("panicked at code/ch07/src/bin/panics.rs:{line}:"), "регистр 40 вне окна"],
    );
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn thread_panic_poisons_mutex() {
    let (st, err) = run("thread");
    assert!(st.success());
    in_order(
        &err,
        &[
            "thread '<unnamed>'",
            "поток обработки упал, держа замок",
            "join: Err(\"поток паниковал\")",
            "замок отравлен, данные: [1, 2, 3, 4]",
            "main завершился",
        ],
    );
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn resume_unwind_continues() {
    let (st, err) = run("resume");
    assert_eq!(st.code(), Some(101));
    in_order(&err, &["ушла дальше", "поймали и отпускаем"]);
    assert!(!err.contains("main завершился"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает процессы")]
fn main_returns_result() {
    let bin = env!("CARGO_BIN_EXE_main_result");
    let ok = Command::new(bin).arg("115200").output().unwrap();
    assert!(ok.status.success());
    assert_eq!(String::from_utf8_lossy(&ok.stdout), "скорость 115200\n");
    let bad = Command::new(bin).arg("115k").output().unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&bad.stderr),
        "Error: ParseIntError { kind: InvalidDigit }\n"
    );
    let none = Command::new(bin).output().unwrap();
    assert_eq!(none.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&none.stderr), "Error: \"нужна скорость порта\"\n");
}
