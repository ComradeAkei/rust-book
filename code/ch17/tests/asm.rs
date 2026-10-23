//! Раздел 17.5: ассемблер, подтверждающий, что `volatile` не сливает
//! обращения. Снят с release-сборки закреплённым `cargo` для x86-64.
//! Глава хостовая, целей Cortex-M у неё нет, поэтому проверяется одна цель.
//!
//! - `ch17_two_plain` — два обычных чтения одного адреса: компилятор
//!   сливает их в одно обращение к памяти.
//! - `ch17_two_volatile` — два `read_volatile` того же адреса: оба
//!   обращения обязаны остаться.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn asm() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(build)
}

fn build() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch17-asm");
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
            n.starts_with("ch17-") && n.ends_with(".s")
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
fn rustc_volatile_reads_are_not_merged() {
    // Обычные чтения слиты в одно обращение к [rdi].
    let plain = body(asm(), "ch17_two_plain");
    let plain_loads = plain.iter().filter(|l| l.contains("[rdi]")).count();
    assert_eq!(plain_loads, 1, "{plain:?}");

    // Два volatile-чтения — два обращения к [rdi].
    let vol = body(asm(), "ch17_two_volatile");
    let vol_loads = vol.iter().filter(|l| l.contains("[rdi]")).count();
    assert_eq!(vol_loads, 2, "{vol:?}");
}
