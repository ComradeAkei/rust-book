//! Раздел 23.3: снимок раскрытия `swap!` с номерами контекстов гигиены в
//! `expanded/swap.out.rs` совпадает с тем, что печатает закреплённый
//! rustc. `-Zunpretty` — флаг nightly; `RUSTC_BOOTSTRAP=1` открывает его на
//! stable только для этой команды осмотра. Номера (`2353#4`) — внутренние
//! номера rustc 1.97, поэтому в задании CI со случайной раскладкой (там
//! только nightly) тест пропускается по префиксу `rustc_`.

use std::path::Path;
use std::process::Command;

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_swap_hygiene_expansion() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("expanded");
    let out = Command::new("rustc")
        .current_dir(&dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["--edition", "2024", "--crate-type", "lib", "-Zunpretty=expanded,hygiene"])
        .arg("swap.rs")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let got = String::from_utf8(out.stdout).unwrap();
    let want = std::fs::read_to_string(dir.join("swap.out.rs")).unwrap();
    assert_eq!(got, want);
    // Суть снимка: у `tmp` из макроса и `tmp` вызывающего разные контексты.
    assert!(got.contains("let tmp /* 2353#4 */ = x /* 2270#0 */;"), "{got}");
    assert!(got.contains("tmp /* 2353#0 */ = tmp /* 2353#4 */;"), "{got}");
}
