//! Проверка кодогенерации главы 21: заявленные в разделе 21.4 x86-инструкции
//! модели памяти действительно эмитятся. Снято с release-сборки примера
//! `emit_probe` закреплённым `cargo rustc --emit asm` в синтаксисе Intel.
//! Тест запускает cargo — под Miri пропускается.
//!
//! - `xchg` — SeqCst-store (полный барьер на x86-64);
//! - `lock` — префикс атомарного read-modify-write (`fetch_add`, CAS).

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn asm() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(build)
}

fn build() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch21-asm");
    let res = Command::new("cargo")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--release", "--example", "emit_probe"])
        .args(["--target", "x86_64-unknown-linux-gnu"])
        .arg("--target-dir")
        .arg(&out)
        .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
        .args(["-C", "llvm-args=-x86-asm-syntax=intel"])
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let d = out.join("x86_64-unknown-linux-gnu/release/examples");
    let file = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("emit_probe-") && n.ends_with(".s")
        })
        .unwrap_or_else(|| panic!("нет файла .s в {}", d.display()));
    std::fs::read_to_string(file).unwrap()
}

/// Мнемоника или префикс присутствует как инструкция (первым словом строки),
/// а не в имени символа или комментарии.
fn has_insn(asm: &str, mnemonic: &str) -> bool {
    asm.lines().any(|l| {
        let t = l.trim();
        !t.starts_with(['.', '#', '@']) && t.split_whitespace().next() == Some(mnemonic)
    })
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn memory_model_instructions_are_emitted() {
    let s = asm();
    assert!(has_insn(s, "xchg"), "нет xchg от SeqCst-store (раздел 21.4)");
    assert!(has_insn(s, "lock"), "нет префикса lock от атомарного RMW (21.4)");
}
