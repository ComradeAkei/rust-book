//! Раздел 31.1: как выглядит упавший тест. Первые два теста падают
//! намеренно и помечены `#[ignore]`; тест `failure_report` запускает этот
//! же тестовый бинарник с `--ignored` и сверяет отчёт.

use ch31::basics::Window;
use std::num::ParseIntError;
use std::process::Command;

// ANCHOR: failing
#[test]
#[ignore = "падает намеренно: отчёт сверяет failure_report"]
fn mean_rounds_to_nearest() {
    let w: Window<4> = "1,2".parse().unwrap();
    assert_eq!(w.mean(), Some(2), "окно {w}");
}

#[test]
#[ignore = "падает намеренно: отчёт сверяет failure_report"]
fn parses_words() -> Result<(), ParseIntError> {
    let w: Window<4> = "1,два".parse()?;
    assert_eq!(w.len(), 2);
    Ok(())
}
// ANCHOR_END: failing

/// Запустить свой же бинарник с аргументами libtest.
fn rerun(args: &[&str]) -> (Option<i32>, String) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe).args(args).env("RUST_BACKTRACE", "0").output().unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code(), text)
}

#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn failure_report() {
    let (code, text) = rerun(&["--ignored", "--test-threads=1"]);
    println!("{text}");
    assert_eq!(code, Some(101), "упавший тест — код выхода 101");
    for line in [
        "test mean_rounds_to_nearest ... FAILED",
        "test parses_words ... FAILED",
        "assertion `left == right` failed: окно 1,2",
        "  left: Some(1)",
        " right: Some(2)",
        "Error: ParseIntError { kind: InvalidDigit }",
        // С --ignored обычные тесты считаются отфильтрованными.
        "test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2 filtered out",
    ] {
        assert!(text.contains(line), "в отчёте нет строки «{line}»");
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn list_and_filter() {
    let (code, text) = rerun(&["--list"]);
    assert_eq!(code, Some(0));
    println!("{text}");
    assert!(text.contains("mean_rounds_to_nearest: test"));
    assert!(text.contains("4 tests, 0 benchmarks"));
    // Фильтр — подстрока имени; --exact требует имя целиком.
    let (_, text) = rerun(&["--list", "--exact", "parses"]);
    assert!(text.contains("0 tests, 0 benchmarks"), "{text}");
}
