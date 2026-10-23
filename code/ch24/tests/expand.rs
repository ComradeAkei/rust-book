//! Разделы 24.2 и 24.4: снимки раскрытия `#[derive(Describe)]` на `Header`
//! и `#[interrupt]` на обработчике UART0 совпадают с тем, что печатает
//! закреплённый rustc. `-Zunpretty=expanded` — флаг nightly;
//! `RUSTC_BOOTSTRAP=1` открывает его на stable только для этой команды
//! осмотра. Вёрстка печати — свойство rustc 1.97, поэтому в задании CI со
//! случайной раскладкой тест пропускается по префиксу `rustc_`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Весь крейт `ch24` после раскрытия макросов.
fn expanded_crate() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch24-expand");
    let out = Command::new("cargo")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["rustc", "--lib", "--profile", "check", "--no-default-features"])
        .arg("--target-dir")
        .arg(&target)
        .args(["--", "-Zunpretty=expanded"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

/// Строки от первой, что начинается с `start` (после отступа), до `n`-й
/// строки, равной `end` на том же отступе; отступ снят.
fn item(text: &str, start: &str, end: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let i = lines
        .iter()
        .position(|l| l.trim_start().starts_with(start))
        .unwrap_or_else(|| panic!("нет строки, начинающейся с {start:?}"));
    let indent = lines[i].len() - lines[i].trim_start().len();
    let close = " ".repeat(indent) + end;
    let j = lines[i..].iter().enumerate().filter(|(_, l)| **l == close).nth(n - 1).unwrap().0 + i;
    lines[i..=j]
        .iter()
        .map(|l| l.get(indent..).unwrap_or("").trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn snapshot(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("expanded").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo и rustc")]
fn rustc_expansions_match_snapshots() {
    let text = expanded_crate();
    // impl трейта и проверка размера из #[describe(size = 8)].
    let describe = item(&text, "impl ::ch24::Describe for Header {", "    };", 1);
    assert_eq!(describe, snapshot("describe_header.rs"));
    // Переходник и переименованная функция пользователя.
    let handler = item(&text, "#[unsafe(export_name = \"UART0\")]", "}", 2);
    assert_eq!(handler, snapshot("interrupt_uart0.rs"));
}
