//! Разделы 10.5 и 10.7: цена обобщений в коде для Cortex-M3, снятая с
//! настоящей сборки закреплённым cargo.
//!
//! - `ch10_typed_send` (typestate) и `ch10_raw_send` (голые записи в
//!   регистры) — одна и та же функция: LLVM слил их в псевдоним.
//! - При `opt-level = "s"` внутренняя необобщённая функция вдвое
//!   сокращает код `slip_*`: тело одно на четыре типа данных.
//!
//! В задании CI со случайной раскладкой (там только nightly) тесты
//! пропускаются по префиксу `rustc_`.

use std::path::{Path, PathBuf};
use std::process::Command;

mod elf {
    include!("../../../tests-shared/elf.rs");
}

/// Собрать библиотеку под thumbv7m-none-eabi закреплённым тулчейном и
/// вернуть её символы.
fn thumb_symbols(name: &str, extra: &[&str]) -> Vec<elf::Symbol> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch10-{name}"));
    let t = "thumbv7m-none-eabi";
    let out = Command::new("cargo")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["build", "--lib", "--release", "--no-default-features", "--target", t])
        .arg("--target-dir")
        .arg(&target)
        .args(extra)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let rlib = std::fs::read(target.join(t).join("release/libch10.rlib")).unwrap();
    elf::rlib_symbols(&rlib)
}

fn find(symbols: &[elf::Symbol], pred: impl Fn(&str) -> bool) -> Vec<&elf::Symbol> {
    symbols.iter().filter(|s| s.func && s.shndx != 0 && pred(&s.name)).collect()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_typestate_is_free() {
    let symbols = thumb_symbols("typestate", &[]);
    let typed = find(&symbols, |n| n == "ch10_typed_send");
    let raw = find(&symbols, |n| n == "ch10_raw_send");
    assert_eq!((typed.len(), raw.len()), (1, 1));
    // Псевдоним: та же секция и то же смещение.
    assert_eq!((typed[0].shndx, typed[0].value), (raw[0].shndx, raw[0].value));
    assert_eq!(typed[0].size, raw[0].size);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_inner_function_shrinks() {
    let symbols = thumb_symbols("opt-s", &["--config", "profile.release.opt-level=\"s\""]);
    // Копии обобщённой функции и вызывающий их ch10_slip_*. В схеме v0
    // путь записан с длинами: `8slip_fat`, `9slip_thin`.
    let total = |path: &str, caller: &str| -> u64 {
        find(&symbols, |n| n.contains(path) || n == caller).iter().map(|s| s.size).sum()
    };
    let fat = total("8slip_fat", "ch10_slip_fat");
    let thin = total("9slip_thin", "ch10_slip_thin");
    let copies = find(&symbols, |n| n.contains("8slip_fat")).len();
    assert_eq!(copies, 4, "четыре типа данных — четыре копии slip_fat");
    assert!(thin * 3 < fat * 2, "slip_thin {thin} байт, slip_fat {fat} байт");
}
