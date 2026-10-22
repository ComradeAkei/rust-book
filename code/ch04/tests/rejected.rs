//! Раздел 4.6: программа из `code/ch04/rejected` отвергается проверкой
//! заимствований на stable и принимается экспериментальным Polonius на
//! nightly. Тесту нужны оба тулчейна: rustc из rust-toolchain.toml и
//! nightly. В задании CI, где установлен только nightly, его пропускают
//! (`--skip polonius`).

use std::path::Path;
use std::process::Command;

/// Собрать файл как библиотеку и вернуть коды ошибок `E…` из вывода.
fn errors(toolchain: Option<&str>, file: &Path, extra: &[&str]) -> Option<Vec<String>> {
    let mut cmd = match toolchain {
        Some(t) => {
            let mut c = Command::new("rustup");
            c.args(["run", t, "rustc"]);
            c
        }
        None => Command::new("rustc"),
    };
    // Без `RUSTUP_TOOLCHAIN` rustup берёт версию из rust-toolchain.toml, даже
    // если тесты запущены через `cargo +nightly`.
    let out = cmd
        .env_remove("RUSTUP_TOOLCHAIN")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata", "-o"])
        .arg(std::env::temp_dir().join("ch04-rejected.rmeta"))
        .args(extra)
        .arg(file)
        .output()
        .ok()?;
    if toolchain.is_some() && !out.status.success() && out.stderr.starts_with(b"error: toolchain") {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stderr);
    let mut codes: Vec<String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("error[").and_then(|r| r.get(..5)).map(str::to_owned))
        .collect();
    codes.dedup();
    assert_eq!(out.status.success(), codes.is_empty(), "{text}");
    Some(codes)
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn polonius_get_or_insert() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("rejected/get_or_insert.rs");
    assert_eq!(errors(None, &file, &[]).unwrap(), ["E0502"]);
    match errors(Some("nightly"), &file, &["-Zpolonius=next"]) {
        Some(codes) => assert!(codes.is_empty(), "Polonius отверг программу: {codes:?}"),
        None => eprintln!("nightly не установлен: проверка Polonius пропущена"),
    }
}
