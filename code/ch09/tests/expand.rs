//! Раздел 9.6: снимок раскрытия `derive` в `expanded/derive.out.rs`
//! совпадает с тем, что печатает закреплённый rustc. `-Zunpretty` —
//! флаг nightly; `RUSTC_BOOTSTRAP=1` открывает его на stable только для
//! этой команды осмотра. В задании CI со случайной раскладкой (там только
//! nightly) тест пропускается: у nightly раскрытие может отличаться.

use std::path::Path;
use std::process::Command;

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_derive_expansion() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("expanded");
    let out = Command::new("rustc")
        .current_dir(&dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["--edition", "2024", "--crate-type", "lib", "-Zunpretty=expanded", "derive.rs"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let want = std::fs::read_to_string(dir.join("derive.out.rs")).unwrap();
    assert_eq!(String::from_utf8(out.stdout).unwrap(), want);
}
