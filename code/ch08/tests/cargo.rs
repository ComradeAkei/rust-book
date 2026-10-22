//! Опыты главы 8, которым нужен сам cargo или rustc: проверка имён в `cfg`
//! (раздел 8.3), унификация возможностей (8.4), типы крейтов (8.6) и
//! столкновения имён без манглинга (8.7). Опыты с cargo — отдельные
//! рабочие пространства рядом с крейтом; тест собирает каждое закреплённым
//! cargo в свой каталог и сверяет результат с текстом главы. Под Miri и в
//! задании CI со случайной раскладкой (там только nightly) эти тесты
//! пропускаются.

use std::os::unix::process::ExitStatusExt;
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

/// `rustc <args>` в каталоге `dup` с выходными файлами в `out`.
fn rustc(out: &Path, args: &[&str]) -> Output {
    Command::new("rustc")
        .current_dir(here("dup"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .args(["--edition", "2024", "--out-dir"])
        .arg(out)
        .args(["-L"])
        .arg(out)
        .args(args)
        .output()
        .unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn cargo_no_mangle_clashes() {
    let out = tmp("dup");
    for lib in ["uart.rs", "dma.rs"] {
        assert!(rustc(&out, &["--crate-type", "rlib", lib]).status.success());
    }
    // Два крейта с одним и тем же именем: ни ошибки, ни предупреждения.
    // Компоновщик берёт из архивов первое определение, второй объектный
    // файл ему уже не нужен. Какое первое — решает порядок rlib в команде
    // компоновки, которой управляет rustc, а не автор.
    let mut sums = Vec::new();
    for main in ["main.rs", "main2.rs"] {
        let r = rustc(&out, &["--extern", "uart", "--extern", "dma", main]);
        assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
        assert!(r.stderr.is_empty(), "{}", String::from_utf8_lossy(&r.stderr));
        sums.push(run(&out.join(main.trim_end_matches(".rs"))));
    }
    assert_eq!(sums, ["4\n", "2\n"]);

    // В одном крейте rustc видит оба определения сам.
    let r = rustc(&out, &["--crate-type", "rlib", "same.rs"]);
    assert!(!r.status.success());
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(err.contains("error: symbol `station_reset` is already defined"), "{err}");

    // Своя malloc заменяет malloc из libc для всего процесса.
    assert!(rustc(&out, &["malloc.rs"]).status.success());
    let r = Command::new(out.join("malloc")).output().unwrap();
    assert_eq!(r.status.signal(), Some(6), "ждали abort");
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(err.starts_with("memory allocation of "), "{err}");
}
