//! Раздел 24.6: при сборке под Cortex-M крейт процедурных макросов
//! собирается для хоста. Тест собирает `ch24` без std под
//! thumbv7m-none-eabi с `cargo build -v` и смотрит на команды rustc и на
//! файлы: у `ch24_macros` — `--crate-type proc-macro` без `--target` и
//! разделяемая библиотека x86-64 в `target/debug`, у `ch24` — `--target
//! thumbv7m-none-eabi` и rlib в `target/thumbv7m-none-eabi`. Запускает cargo,
//! поэтому префикс `rustc_` и ignore под Miri.

use std::path::PathBuf;
use std::process::Command;

/// Команда rustc для крейта `name` из вывода `cargo build -v`.
fn rustc_line<'a>(log: &'a str, name: &str) -> &'a str {
    let key = format!("--crate-name {name} ");
    log.lines()
        .find(|l| l.contains("Running `") && l.contains(&key))
        .unwrap_or_else(|| panic!("нет команды для {name}:\n{log}"))
}

fn find(dir: &PathBuf, prefix: &str, ext: &str) -> PathBuf {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with(prefix) && n.ends_with(ext)
        })
        .unwrap_or_else(|| panic!("нет {prefix}*{ext} в {}", dir.display()))
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_proc_macro_is_built_for_host() {
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch24-host");
    let t = "thumbv7m-none-eabi";
    let out = Command::new("cargo")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["build", "-v", "--lib", "--no-default-features", "--target", t])
        .arg("--target-dir")
        .arg(&target)
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{log}");
    // Пустую сборку cargo не печатает: каталог свой, но мог остаться от
    // прошлого прогона.
    if log.contains("Running `") {
        let macros = rustc_line(&log, "ch24_macros");
        assert!(macros.contains("--crate-type proc-macro"), "{macros}");
        assert!(!macros.contains("--target"), "{macros}");
        let lib = rustc_line(&log, "ch24");
        assert!(lib.contains(&format!("--target {t}")), "{lib}");
    }
    // Макросы — ELF x86-64 (e_machine = 62) рядом с хостовыми артефактами.
    let so = find(&target.join("debug/deps"), "libch24_macros-", ".so");
    let elf = std::fs::read(&so).unwrap();
    assert_eq!(&elf[..4], b"\x7fELF");
    assert_eq!(u16::from_le_bytes([elf[18], elf[19]]), 62, "{}", so.display());
    // Библиотека — под Cortex-M.
    find(&target.join(t).join("debug/deps"), "libch24-", ".rlib");
}
