//! Раздел 26.2: программа `code/ch26/start` — процесс Linux без libc.
//! Тест собирает её закреплённым cargo (у крейта свой workspace и свои
//! профили), запускает и разбирает заголовки ELF.
//!
//! Под Miri тесты пропускаются, в задании CI со случайной раскладкой —
//! по префиксу `rustc_`.

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Собрать `start` в release в своём каталоге `dir`. `env` — переменные
/// окружения сборки, `features` — возможности крейта.
fn build(dir: &str, env: &[(&str, &str)], features: &[&str]) -> (Output, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("start");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir);
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&src)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_TARGET")
        .args(["build", "--release", "--color", "never", "--target-dir"])
        .arg(&out);
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    (cmd.output().unwrap(), out.join("release/ch26-start"))
}

fn built(dir: &str, env: &[(&str, &str)], features: &[&str]) -> PathBuf {
    let (res, exe) = build(dir, env, features);
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    exe
}

/// Тип файла ELF и типы сегментов из заголовков программы (ELF64 LE).
fn elf_kinds(path: &Path) -> (u16, Vec<u32>) {
    let b = std::fs::read(path).unwrap();
    assert_eq!(&b[..4], b"\x7fELF");
    let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let phoff = u64::from_le_bytes(b[0x20..0x28].try_into().unwrap()) as usize;
    let (phentsize, phnum) = (u16_at(0x36) as usize, u16_at(0x38) as usize);
    (u16_at(0x10), (0..phnum).map(|i| u32_at(phoff + i * phentsize)).collect())
}

const ET_EXEC: u16 = 2;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_start_reads_the_initial_stack() {
    let exe = built("start", &[], &[]);
    let run = Command::new(&exe)
        .args(["один", "два"])
        .env_clear()
        .env("A", "1")
        .env("B", "2")
        .output()
        .unwrap();
    assert!(run.status.success(), "{:?}", run.status);
    let text = String::from_utf8(run.stdout).unwrap();
    assert!(text.starts_with("argc = 3\nargv[0] = "), "{text}");
    assert!(text.contains("argv[1] = один\nargv[2] = два\n"), "{text}");
    assert!(text.contains("переменных окружения: 2\n"), "{text}");
    assert!(text.contains("AT_PAGESZ = 4096\n"), "{text}");
    assert!(text.contains("AT_RANDOM: 16 случайных байт по адресу 0x"), "{text}");
    assert!(text.contains(", кратно 16\n"), "{text}");
    // Ядро сообщает адрес входа в auxv, и это наш _start.
    let entry = text.lines().find(|l| l.starts_with("AT_ENTRY = ")).unwrap();
    let (at, start) = entry["AT_ENTRY = ".len()..].split_once(", _start = ").unwrap();
    assert_eq!(at, start);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_start_is_static_exec() {
    let exe = built("start", &[], &[]);
    let (kind, segments) = elf_kinds(&exe);
    assert_eq!(kind, ET_EXEC);
    assert!(!segments.contains(&PT_INTERP), "{segments:?}");
    assert!(!segments.contains(&PT_DYNAMIC), "{segments:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_start_without_static_gets_interpreter() {
    let exe = built("start-dyn", &[("CH26_DYNAMIC", "1")], &[]);
    let (_, segments) = elf_kinds(&exe);
    assert!(segments.contains(&PT_INTERP), "{segments:?}");
    // До нашего _start выполнится ld-linux-x86-64.so.2, но программа та же.
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success());
    assert!(String::from_utf8_lossy(&run.stdout).starts_with("argc = 1\n"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_start_without_lto_needs_personality() {
    let (res, _) = build("start-nolto", &[("CARGO_PROFILE_RELEASE_LTO", "false")], &[]);
    assert!(!res.status.success());
    let err = String::from_utf8_lossy(&res.stderr);
    assert!(err.contains("undefined symbol: rust_eh_personality"), "{err}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_observed_misaligned_stack_faults_on_movaps() {
    let exe = built("start-misalign", &[], &["misalign"]);
    let run = Command::new(&exe).output().unwrap();
    // SIGSEGV: movaps по адресу, не кратному 16, — общая ошибка защиты.
    assert_eq!(run.status.signal(), Some(11), "{:?}", run.status);
    assert_eq!(run.stdout, "argc = ".as_bytes());
}
