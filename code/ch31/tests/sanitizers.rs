//! Раздел 31.5: опыты программы `san` под ASan и TSan. Тест сам собирает
//! её nightly-компилятором с `-Zsanitizer` и `-Zbuild-std` в отдельный
//! каталог и сверяет отчёты санитайзеров. Без nightly с компонентом
//! rust-src опыты пропускаются; `CH31_REQUIRE_SANITIZERS` делает пропуск
//! ошибкой — так тест запускает CI.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::OnceLock;

const TARGET: &str = "x86_64-unknown-linux-gnu";

fn require() -> bool {
    std::env::var_os("CH31_REQUIRE_SANITIZERS").is_some()
}

/// `cargo +nightly` без переменных, которые внешний cargo задал тесту, и
/// без кодов цвета: текст ошибок сборки сравнивается, а в CI задан
/// CARGO_TERM_COLOR=always.
fn nightly_cargo() -> Command {
    let mut c = Command::new("cargo");
    c.arg("+nightly").current_dir(env!("CARGO_MANIFEST_DIR")).env("CARGO_TERM_COLOR", "never");
    for v in ["CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS", "RUSTUP_TOOLCHAIN", "RUSTC", "RUSTDOC"] {
        c.env_remove(v);
    }
    c
}

/// Собрать `san` с санитайзером; `None` — нет nightly или rust-src.
fn build(sanitizer: &str) -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch31-{sanitizer}"));
    // ANCHOR: build
    let out = nightly_cargo()
        .env("RUSTFLAGS", format!("-Zsanitizer={sanitizer}"))
        .args(["build", "-q", "-Zbuild-std", "--target", TARGET, "--release", "--bin", "san"])
        .arg("--target-dir")
        .arg(&dir)
        .output()
        .unwrap();
    // ANCHOR_END: build
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!require(), "сборка с -Zsanitizer={sanitizer} не удалась:\n{err}");
        eprintln!(
            "пропуск: сборка с -Zsanitizer={sanitizer} не удалась (нужен nightly с rust-src)"
        );
        return None;
    }
    Some(dir.join(TARGET).join("release").join("san"))
}

fn asan() -> Option<&'static PathBuf> {
    static EXE: OnceLock<Option<PathBuf>> = OnceLock::new();
    EXE.get_or_init(|| build("address")).as_ref()
}

fn tsan() -> Option<&'static PathBuf> {
    static EXE: OnceLock<Option<PathBuf>> = OnceLock::new();
    EXE.get_or_init(|| build("thread")).as_ref()
}

fn run(exe: &PathBuf, what: &str) -> (Option<i32>, String) {
    let Output { status, stdout, stderr } = Command::new(exe).arg(what).output().unwrap();
    let text = String::from_utf8_lossy(&stdout).into_owned() + &String::from_utf8_lossy(&stderr);
    println!("== san {what}: код {:?}\n{text}", status.code());
    (status.code(), text)
}

fn expect_all(text: &str, lines: &[&str]) {
    for l in lines {
        assert!(text.contains(l), "в отчёте нет «{l}»");
    }
}

/// Без санитайзера ошибки молчат. Программа собрана в профиле dev: там
/// `get_unchecked` проверяет своё предусловие сам (ub_checks).
#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn without_sanitizers() {
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_san"));
    let (code, text) = run(&exe, "uaf");
    assert_eq!(code, Some(0));
    assert!(text.starts_with("первый элемент: "));
    assert_eq!(run(&exe, "ffi"), (Some(0), "последний: 15\n".into()));
    assert_eq!(run(&exe, "leak"), (Some(0), "вектор забыт\n".into()));
    let (code, text) = run(&exe, "race");
    assert_eq!(code, Some(0));
    assert!(text.contains(" из 200000"));
    let (code, text) = run(&exe, "relaxed");
    assert_eq!((code, text.as_str()), (Some(0), "сумма: 49995000\n"));
    assert_eq!(run(&exe, "once"), (Some(0), "999\n".into()));
    let (code, text) = run(&exe, "overflow");
    assert_eq!(code, None, "abort — сигнал, а не код выхода");
    expect_all(
        &text,
        &[
            "unsafe precondition(s) violated: slice::get_unchecked requires that the index is within the slice",
            "thread caused non-unwinding panic. aborting.",
        ],
    );
}

// ANCHOR: asan_checks
#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn address_sanitizer() {
    let Some(exe) = asan() else { return };
    let (code, text) = run(exe, "uaf");
    assert_eq!(code, Some(1));
    expect_all(
        &text,
        &["ERROR: AddressSanitizer: heap-use-after-free", "READ of size 4", "in san::uaf"],
    );
    let (code, text) = run(exe, "overflow");
    assert_eq!(code, Some(1));
    expect_all(
        &text,
        &["ERROR: AddressSanitizer: heap-buffer-overflow", "READ of size 1", "in san::overflow"],
    );
    // Ошибка в C: ASan видит запись, потому что build.rs собрал fill.c
    // с -fsanitize=address.
    let (code, text) = run(exe, "ffi");
    assert_eq!(code, Some(1));
    expect_all(
        &text,
        &[
            "ERROR: AddressSanitizer: heap-buffer-overflow",
            "WRITE of size 1",
            "in fill_counter",
            "code/ch31/c/fill.c:9",
            "in san::ffi",
        ],
    );
    let (code, text) = run(exe, "leak");
    assert_eq!(code, Some(1));
    expect_all(
        &text,
        &[
            "ERROR: LeakSanitizer: detected memory leaks",
            "Direct leak of 1000 byte(s) in 1 object(s) allocated from:",
        ],
    );
}
// ANCHOR_END: asan_checks

// ANCHOR: tsan_checks
#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn thread_sanitizer() {
    let Some(exe) = tsan() else { return };
    let (code, text) = run(exe, "race");
    assert_eq!(code, Some(66), "TSan завершает программу с кодом 66");
    expect_all(
        &text,
        &[
            "WARNING: ThreadSanitizer: data race",
            "san::race::{closure#0}::{closure#0}",
            "ThreadSanitizer: reported 1 warnings",
        ],
    );
    // Очередь главы 21 чиста, а её вариант со всеми Relaxed — нет, хотя
    // сумма на x86-64 сходится.
    assert_eq!(run(exe, "spsc"), (Some(0), "сумма: 49995000\n".into()));
    let (code, text) = run(exe, "relaxed");
    assert_eq!(code, Some(66));
    expect_all(
        &text,
        &[
            "WARNING: ThreadSanitizer: data race",
            "<ch31::spsc::Spsc<u32, 8, ch31::spsc::AllRelaxed>>::pop",
            "сумма: 49995000",
        ],
    );
    // Синхронизация внутри std: TSan видит её, потому что std собрана с ним.
    assert_eq!(run(exe, "once"), (Some(0), "999\n".into()));
}
// ANCHOR_END: tsan_checks

/// Без -Zbuild-std: ASan собирается поверх готовой std и ловит ошибки в
/// нашем коде, TSan nightly собирать отказывается — std без санитайзера.
/// Если отказ заглушить, TSan не видит синхронизации внутри std и находит
/// гонку в правильной программе.
#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn without_build_std() {
    let build = |flags: &str| {
        let sanitizer = flags.split_whitespace().next().unwrap();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch31-{sanitizer}-std"));
        let out = nightly_cargo()
            .env("RUSTFLAGS", format!("-Zsanitizer={flags}"))
            .args(["build", "-q", "--target", TARGET, "--release", "--bin", "san", "--target-dir"])
            .arg(&dir)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        (out.status.success(), err, dir.join(TARGET).join("release").join("san"))
    };
    let (ok, err, exe) = build("address");
    if !ok {
        assert!(!require(), "сборка с ASan без -Zbuild-std не удалась:\n{err}");
        eprintln!("пропуск: нет nightly");
        return;
    }
    let (code, text) = run(&exe, "uaf");
    assert_eq!(code, Some(1));
    expect_all(&text, &["ERROR: AddressSanitizer: heap-use-after-free"]);
    let (ok, err, _) = build("thread");
    println!("{err}");
    assert!(!ok, "TSan без -Zbuild-std собрался");
    expect_all(
        &err,
        &[
            "error: mixing `-Zsanitizer` will cause an ABI mismatch",
            "`-Zsanitizer=thread` in this crate is incompatible with `-Zsanitizer` being unset in dependency `core`",
        ],
    );
    // ANCHOR: mismatch
    let (ok, err, exe) = build("thread -Cunsafe-allow-abi-mismatch=sanitizer");
    assert!(ok, "TSan с -Cunsafe-allow-abi-mismatch не собрался:\n{err}");
    let (code, text) = run(&exe, "once");
    assert_eq!(code, Some(66), "ложная гонка в правильной программе");
    expect_all(&text, &["WARNING: ThreadSanitizer: data race", "999\n"]);
    // ANCHOR_END: mismatch
}
