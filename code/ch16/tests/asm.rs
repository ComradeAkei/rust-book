//! Раздел 16.3: ассемблер, который глава приводит в тексте, снят с
//! release-сборки закреплённым cargo для x86-64. Глава 16 — про `unsafe`
//! и UB на хосте, целей Cortex-M у неё нет, поэтому проверяется одна цель.
//!
//! - `ch16_checked` — индекс с проверкой границ: сравнение и ветка на
//!   `panic_bounds_check`.
//! - `ch16_unchecked` — тот же доступ без проверки: одна загрузка, без
//!   ветки и вызова.
//! - `ch16_assume_even` — обещание чётности сводит тело к `return n`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn asm() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(build)
}

fn build() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch16-asm");
    let res = Command::new("cargo")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--release", "--lib", "--target", "x86_64-unknown-linux-gnu"])
        .args(["--target-dir"])
        .arg(&out)
        .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
        .args(["-C", "llvm-args=-x86-asm-syntax=intel"])
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let deps = out.join("x86_64-unknown-linux-gnu/release/deps");
    let file = std::fs::read_dir(&deps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("ch16-") && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции `name` (no_mangle): без директив, меток и
/// комментариев, пробелы сжаты.
fn body(asm: &str, name: &str) -> Vec<String> {
    let start =
        asm.lines().position(|l| l == format!("{name}:")).unwrap_or_else(|| panic!("нет {name}"));
    asm.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with(".Lfunc_end"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_bounds_check_costs_a_branch() {
    let checked = body(asm(), "ch16_checked");
    // Проверка границ: сравнение индекса с длиной и вызов паники.
    assert!(checked.iter().any(|l| l == "cmp rdx, rsi"), "{checked:?}");
    assert!(checked.iter().any(|l| l.contains("panic_bounds_check")), "{checked:?}");

    let unchecked = body(asm(), "ch16_unchecked");
    // Без проверки: одна загрузка, ни сравнения с длиной, ни вызова.
    assert!(unchecked.iter().any(|l| l == "mov eax, dword ptr [rdi + 4*rdx]"), "{unchecked:?}");
    assert!(
        !unchecked.iter().any(|l| l.starts_with("cmp") || l.starts_with("call")),
        "{unchecked:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_assume_propagates_backward() {
    let b = body(asm(), "ch16_assume_even");
    // Обещание чётности: тело свёрнуто до «вернуть n», без деления.
    assert!(b.iter().any(|l| l == "mov eax, edi"), "{b:?}");
    assert!(!b.iter().any(|l| l.starts_with("div") || l.starts_with("and")), "{b:?}");
}
