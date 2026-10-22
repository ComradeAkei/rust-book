//! Опыты главы 8, которым нужен сам cargo: проверка имён в `cfg`
//! (раздел 8.3), унификация возможностей (8.4) и типы крейтов (8.6).
//! Каждый опыт — отдельное рабочее пространство рядом с крейтом; тест
//! собирает его закреплённым cargo в свой каталог и сверяет результат с
//! текстом главы. Под Miri и в задании CI со случайной раскладкой (там
//! только nightly) эти тесты пропускаются.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn here(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(dir)
}

fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch08-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `cargo <args>` в каталоге `dir` с целевым каталогом `target`.
fn cargo(dir: &str, target: &Path, args: &[&str]) -> Output {
    let out = Command::new(env!("CARGO"))
        .current_dir(here(dir))
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env("CARGO_TERM_COLOR", "never")
        .args(args)
        .arg("--target-dir")
        .arg(target)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    out
}

fn run(bin: &Path) -> String {
    let out = Command::new(bin).output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_typo_warns() {
    let out = cargo("typo", &tmp("typo"), &["check"]);
    let err = String::from_utf8_lossy(&out.stderr);
    for line in [
        "warning: unexpected `cfg` condition value: `dam`",
        "help: there is a expected value with a similar name: `\"dma\"`",
        "warning: unexpected `cfg` condition value: `linxu`",
        "help: there is a expected value with a similar name: `\"linux\"`",
    ] {
        assert!(err.contains(line), "нет строки «{line}»:\n{err}");
    }
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_unify_changes_behaviour() {
    let alone = tmp("unify-alone");
    cargo("unify", &alone, &["build", "-q", "-p", "app-a"]);
    assert_eq!(run(&alone.join("debug/app-a")), "кадр 1: 42\n");

    let all = tmp("unify-all");
    cargo("unify", &all, &["build", "-q", "--workspace"]);
    assert_eq!(run(&all.join("debug/app-a")), "кадр 1: 0x2a\n");
    assert_eq!(run(&all.join("debug/app-b")), "кадр 2: 0xff\n");
}

/// Имена определённых глобальных символов из вывода `nm`.
fn nm(args: &[&str], file: &Path) -> Vec<String> {
    let out = Command::new("nm").args(args).arg(file).output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() == 3 && f[1] == "T").then(|| f[2].to_string())
        })
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_crate_types() {
    let target = tmp("ffi");
    cargo("ffi", &target, &["build", "-q", "--release"]);
    let rel = target.join("release");

    // cdylib экспортирует только то, что помечено no_mangle.
    assert_eq!(nm(&["-D", "--defined-only"], &rel.join("libsensorlib.so")), ["sensor_scale"]);

    // staticlib несёт с собой std, core и compiler_builtins целиком.
    let members = Command::new("ar").arg("t").arg(rel.join("libsensorlib.a")).output().unwrap();
    let members = String::from_utf8(members.stdout).unwrap();
    for krate in ["std-", "core-", "alloc-", "compiler_builtins-", "sensorlib."] {
        assert!(members.lines().any(|m| m.starts_with(krate)), "нет {krate}*:\n{members}");
    }
    let globals = nm(&["-g", "--defined-only"], &rel.join("libsensorlib.a"));
    assert!(globals.iter().any(|s| s == "sensor_scale"));
    assert!(globals.len() > 1000, "глобальных символов всего {}", globals.len());

    // rlib — метаданные и объектный код самого крейта, без std.
    let members = Command::new("ar").arg("t").arg(rel.join("libsensorlib.rlib")).output().unwrap();
    let members = String::from_utf8(members.stdout).unwrap();
    assert!(members.lines().any(|m| m == "lib.rmeta"), "{members}");
    assert!(!members.contains("std-"), "{members}");
}
