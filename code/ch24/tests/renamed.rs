//! Раздел 24.4: у процедурного макроса нет `$crate`. Вложенный крейт
//! `renamed/` подключает `ch24` под именем `station`: с
//! `#[describe(crate = station)]` он собирается, а структура без атрибута
//! (возможность `broken`) даёт E0433 — пути `::ch24` в нём нет. Тест
//! запускает cargo, поэтому префикс `rustc_` и ignore под Miri.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn check(features: &[&str]) -> Output {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("renamed/Cargo.toml");
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch24-renamed");
    Command::new("cargo")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .arg("check")
        // Текст ошибки сравнивается ниже: без кодов цвета, даже если
        // снаружи задан CARGO_TERM_COLOR=always (так в CI).
        .args(["--color", "never"])
        .arg("--target-dir")
        .arg(&target)
        .arg("--manifest-path")
        .arg(&manifest)
        .args(features)
        .output()
        .unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_crate_path_attribute() {
    let ok = check(&[]);
    assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
    let broken = check(&["--features", "broken"]);
    let err = String::from_utf8_lossy(&broken.stderr);
    assert!(!broken.status.success(), "{err}");
    assert!(err.contains("error[E0433]: cannot find `ch24` in the crate root"), "{err}");
    assert!(err.contains("#[derive(station::Describe)]"), "{err}");
}
