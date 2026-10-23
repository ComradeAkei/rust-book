//! Разделы 15.1, 15.5 и 15.8: ассемблер, который глава приводит в тексте,
//! снятый с release-сборки закреплённым cargo для x86-64 и трёх целей
//! Cortex-M.
//!
//! - `Vec::push` — сравнение длины с ёмкостью, запись и инкремент; рост —
//!   вызов `grow_one` вне быстрого пути.
//! - `VecDeque::get` — сравнение и вычитание ёмкости, без деления.
//! - `heapless::Vec::push` — сравнение с константой, без вызовов.
//!
//! Регистры и порядок полей — наблюдение для rustc 1.97, не гарантия. В
//! задании CI со случайной раскладкой (там только nightly) тесты
//! пропускаются по префиксу `rustc_`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Ассемблер библиотеки для цели `target`, собранный один раз на прогон.
fn asm(target: &'static str) -> &'static str {
    static CACHE: OnceLock<Mutex<HashMap<&str, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut map = cache.lock().unwrap();
    map.entry(target).or_insert_with(|| Box::leak(build(target).into_boxed_str()))
}

fn build(target: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch15-asm-{target}"));
    let mut cmd = Command::new("cargo");
    cmd.current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--release", "--lib", "--target", target, "--target-dir"])
        .arg(&out_dir);
    if target.starts_with("thumb") {
        cmd.args(["--no-default-features", "--features", "alloc"]);
    }
    cmd.args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"]);
    if target.starts_with("x86_64") {
        cmd.args(["-C", "llvm-args=-x86-asm-syntax=intel"]);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let deps = out_dir.join(target).join("release/deps");
    let file = std::fs::read_dir(&deps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("ch15-") && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции, метка которой содержит все `parts`: без директив,
/// меток и комментариев, пробелы и табуляции сжаты до одного пробела. Для
/// функций с `no_mangle` достаточно имени.
fn body(asm: &str, parts: &[&str]) -> Vec<String> {
    let start = asm
        .lines()
        .position(|l| {
            l.ends_with(':')
                && !l.starts_with('.')
                && (parts.len() > 1 || l == format!("{}:", parts[0]))
                && parts.iter().all(|p| l.contains(p))
        })
        .unwrap_or_else(|| panic!("нет функции {parts:?}"));
    asm.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with(".Lfunc_end"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
        .collect()
}

fn has(body: &[String], insn: &str) -> bool {
    body.iter().any(|l| l == insn)
}

/// Сколько инструкций начинается с `prefix`.
fn count(body: &[String], prefix: &str) -> usize {
    body.iter().filter(|l| l.starts_with(prefix)).count()
}

const X86: &str = "x86_64-unknown-linux-gnu";
const M0: &str = "thumbv6m-none-eabi";
const M3: &str = "thumbv7m-none-eabi";
const M4F: &str = "thumbv7em-none-eabihf";

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_push_fast_path() {
    let x86 = body(asm(X86), &["ch15_push"]);
    assert!(has(&x86, "cmp r14, qword ptr [rdi]"), "{x86:?}");
    assert!(has(&x86, "mov dword ptr [rax + 4*r14], esi"), "{x86:?}");
    assert!(has(&x86, "inc r14"), "{x86:?}");
    assert_eq!(count(&x86, "call"), 1, "{x86:?}");
    assert!(x86.iter().any(|l| l.contains("grow_one") && l.contains("@GOTPCREL")), "{x86:?}");
    for t in [M0, M3, M4F] {
        let b = body(asm(t), &["ch15_push"]);
        assert_eq!(
            b.iter().filter(|l| l.starts_with("bl ") && l.contains("grow_one")).count(),
            1,
            "{t}: {b:?}"
        );
        // На M0 у Thumb-1 нет адресации со сдвигом регистра: сдвиг отдельной
        // инструкцией.
        let scaled = if t == M0 {
            has(&b, "lsls r2, r5, #2") && has(&b, "str r1, [r3, r2]")
        } else {
            b.iter().any(|l| l.starts_with("str") && l.contains("lsl #2]"))
        };
        assert!(scaled, "{t}: {b:?}");
    }
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_deque_index_without_division() {
    for t in [X86, M0, M3, M4F] {
        let b = body(asm(t), &["ch15_deque_get"]);
        assert!(!b.iter().any(|l| l.contains("div") || l.starts_with("bl ")), "{t}: {b:?}");
        assert!(b.iter().any(|l| l.starts_with("sub")), "{t}: {b:?}");
    }
    let x86 = body(asm(X86), &["ch15_deque_get"]);
    assert!(x86.iter().any(|l| l.starts_with("cmovae")), "{x86:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_heapless_push_no_calls() {
    let x86 = body(asm(X86), &["ch15_hpush"]);
    assert!(has(&x86, "cmp rax, 7"), "{x86:?}");
    for t in [X86, M0, M3, M4F] {
        let b = body(asm(t), &["ch15_hpush"]);
        assert!(!b.iter().any(|l| l.starts_with("call") || l.starts_with("bl ")), "{t}: {b:?}");
    }
    let m3 = body(asm(M3), &["ch15_hpush"]);
    assert!(has(&m3, "cmp r2, #7") && has(&m3, "itttt ls"), "{m3:?}");
}
