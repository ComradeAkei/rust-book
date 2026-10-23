//! Разделы 14.1 и 14.5: примеры, поведение которых зависит от профиля
//! сборки. Тесты собирают и запускают их закреплённым cargo в отладочном
//! профиле и в release.
//!
//! - `bigbox new`: `Box::new([0; 16 МиБ])` в отладочной сборке переполняет
//!   стек главного потока, в release — нет; `zeroed` и `vec` не трогают стек
//!   в обеих. `bigbox seq` — массив из `from_fn` — переполняет стек и в
//!   release: это наблюдение за оптимизатором, тест назван `cargo_observed_`.
//! - `sneaky`: запись через указатель из `&u32` — в отладочной сборке 100,
//!   в release 2. Это наблюдение за неопределённым поведением: ответ
//!   release зависит от оптимизатора, и тесты названы `cargo_observed_`.
//!
//! В задании CI со случайной раскладкой тесты пропускаются по префиксу
//! `cargo_`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(example: &str, release: bool, args: &[&str]) -> Output {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch14-run");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env_remove("RUST_BACKTRACE")
        .args(["run", "-q", "--target-dir"])
        .arg(&target)
        .args(["--example", example]);
    if release {
        cmd.arg("--release");
    }
    cmd.arg("--").args(args);
    cmd.output().unwrap()
}

fn text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr)
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_bigbox() {
    let debug = run("bigbox", false, &["new"]);
    assert!(!debug.status.success());
    assert!(
        text(&debug).contains("thread 'main'") && text(&debug).contains("has overflowed its stack")
    );
    let release = run("bigbox", true, &["new"]);
    assert!(release.status.success(), "{}", text(&release));
    assert_eq!(text(&release), "new: 16 МиБ в куче, сумма 0\n");
    for how in ["zeroed", "vec"] {
        for release in [false, true] {
            let o = run("bigbox", release, &[how]);
            assert_eq!(text(&o), format!("{how}: 16 МиБ в куче, сумма 0\n"), "release = {release}");
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_observed_bigbox_from_fn() {
    for release in [false, true] {
        let o = run("bigbox", release, &["seq"]);
        assert!(!o.status.success(), "release = {release}");
        assert!(text(&o).contains("has overflowed its stack"), "{}", text(&o));
    }
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_observed_sneaky() {
    assert_eq!(text(&run("sneaky", false, &[])), "twice(&a) = 100\n");
    assert_eq!(text(&run("sneaky", true, &[])), "twice(&a) = 2\n");
}
