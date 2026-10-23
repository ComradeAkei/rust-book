//! Раздел 25.3: когда rustc вычисляет константу. Фрагмент
//! `fragments/when.rs` собирается закреплённым rustc по одному случаю с
//! `--emit metadata` (как `cargo check`) и `--emit link` (как
//! `cargo build`).
//!
//! Гарантия Reference: свободная константа вычисляется всегда. Остальное
//! — наблюдение rustc 1.97: Reference обещает вычислить блок `const`,
//! только если он исполняется, а когда — не уточняет.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Собрать случай `case`; `true` — rustc сообщил об ошибке E0080.
fn fails(case: &str, emit: &str) -> bool {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("fragments/when.rs");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch25-when-{case}-{emit}"));
    std::fs::create_dir_all(&dir).unwrap();
    let out = Command::new("rustc")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .args(["--edition", "2024", "--crate-type", "lib", "--crate-name", "when"])
        .args(["--emit", emit, "--cfg", &format!("case=\"{case}\""), "--out-dir"])
        .arg(&dir)
        .arg(&src)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    if out.status.success() {
        return false;
    }
    assert!(err.contains("error[E0080]") && err.contains("small: нужно меньше 10"), "{err}");
    true
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_free_const_fails_check_and_build() {
    assert!(fails("item", "metadata"));
    assert!(fails("item", "link"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_observed_inline_const_fails_only_build() {
    assert!(!fails("block", "metadata"), "cargo check заметил ошибку в блоке const");
    assert!(fails("block", "link"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_observed_generic_inline_const_needs_call() {
    assert!(!fails("generic", "metadata"));
    assert!(!fails("generic", "link"));
}
