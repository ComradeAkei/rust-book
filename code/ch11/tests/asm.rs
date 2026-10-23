//! Разделы 11.1–11.7: ассемблер, который глава приводит в тексте, снятый с
//! release-сборки закреплённым cargo для x86-64 и Cortex-M3.
//!
//! - Вызов через `&dyn Sensor` — загрузка слова 3 таблицы и переход.
//! - Таблица `Thermo` для `Sensor`: ноль на месте `drop`, размер 2,
//!   выравнивание 2, затем `read` и `name`.
//! - Девиртуализация: если тип виден, вызова нет вовсе.
//! - Приведение к первому супертрейту не трогает таблицу, ко второму —
//!   читает указатель из неё.
//!
//! Раскладка таблицы и регистры — наблюдение для rustc 1.97, не гарантия.
//! В задании CI со случайной раскладкой (там только nightly) тесты
//! пропускаются по префиксу `rustc_`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Ассемблер библиотеки для цели `target`, собранный один раз на прогон.
fn asm(target: &'static str) -> &'static str {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<&str, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut map = cache.lock().unwrap();
    map.entry(target).or_insert_with(|| Box::leak(build(target).into_boxed_str()))
}

fn build(target: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch11-asm-{target}"));
    let mut cmd = Command::new("cargo");
    cmd.current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--release", "--lib", "--target", target, "--target-dir"])
        .arg(&out_dir);
    if target.starts_with("thumb") {
        cmd.arg("--no-default-features");
    }
    cmd.args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"]);
    if target.starts_with("x86_64") {
        cmd.args(["-C", "llvm-args=-x86-asm-syntax=intel"]);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let deps = out_dir.join(target).join("release/deps");
    let file = std::fs::read_dir(&deps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("ch11-") && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции без директив, меток и комментариев; табуляции
/// заменены пробелами.
fn body(asm: &str, name: &str) -> Vec<String> {
    let start = asm
        .lines()
        .position(|l| l == format!("{name}:"))
        .unwrap_or_else(|| panic!("нет функции {name}"));
    asm.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with(".Lfunc_end"))
        .map(|l| l.trim().replace('\t', " "))
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
        .collect()
}

fn has(body: &[String], insn: &str) -> bool {
    body.iter().any(|l| l == insn)
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_dyn_calls_x86() {
    let a = asm("x86_64-unknown-linux-gnu");
    assert_eq!(body(a, "ch11_read_dyn"), ["jmp qword ptr [rsi + 24]"]);
    assert_eq!(body(a, "ch11_name_dyn"), ["jmp qword ptr [rsi + 32]"]);
    let drop = body(a, "ch11_drop_box");
    assert!(has(&drop, "mov rax, qword ptr [rsi]") && has(&drop, "call rax"), "{drop:?}");
    assert!(drop.iter().any(|l| l.contains("__rust_dealloc")), "{drop:?}");
    let rc = body(a, "ch11_rc_read");
    assert!(has(&rc, "and rdx, -16") && has(&rc, "jmp qword ptr [rcx + 24]"), "{rc:?}");
    assert!(has(&body(a, "ch11_log_dyn"), "mov rax, qword ptr [rsi + 32]"));
    // Таблица неизменна (`!invariant.load`): адрес `read` читается один раз
    // на два вызова.
    let twice = body(a, "ch11_read_twice");
    assert_eq!(twice.iter().filter(|l| l.contains("[rsi + 24]")).count(), 1, "{twice:?}");
    assert_eq!(twice.iter().filter(|l| l.starts_with("call")).count(), 2, "{twice:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_devirtualization_x86() {
    let a = asm("x86_64-unknown-linux-gnu");
    assert_eq!(body(a, "ch11_devirt_local"), ["mov eax, 2150", "ret"]);
    assert_eq!(body(a, "ch11_devirt_static"), ["mov eax, 2162", "ret"]);
    assert!(has(&body(a, "ch11_devirt_hidden"), "call qword ptr [rax + 24]"));
    assert!(has(&body(a, "ch11_poll_dyn"), "call qword ptr [rax + 24]"));
    let e = body(a, "ch11_read_enum");
    assert!(e.iter().all(|l| !l.contains("call") && !l.contains("qword ptr")), "{e:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_upcasting_x86() {
    let a = asm("x86_64-unknown-linux-gnu");
    let first = body(a, "ch11_upcast_first");
    assert!(first.iter().all(|l| !l.contains("ptr [")), "первый супертрейт: {first:?}");
    assert!(has(&body(a, "ch11_upcast_second"), "mov rdx, qword ptr [rsi + 48]"));
    assert!(has(&body(a, "ch11_as_fan"), "mov rax, qword ptr [rsi + 64]"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_dyn_calls_thumb() {
    let a = asm("thumbv7m-none-eabi");
    let read = body(a, "ch11_read_dyn");
    assert!(has(&read, "ldr r1, [r1, #12]") && has(&read, "bx r1"), "{read:?}");
    assert!(body(a, "ch11_devirt_local").iter().any(|l| l == "movw r0, #2150"));
    let poll = body(a, "ch11_poll_dyn");
    assert!(has(&poll, "ldr r1, [r1, #12]") && has(&poll, "blx r1"), "{poll:?}");
}

/// Таблица, которую строит `ch11_as_dyn`: метка из `lea`, затем данные.
#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_vtable_layout() {
    for (target, words, ptr) in [
        (
            "x86_64-unknown-linux-gnu",
            r#".asciz "\000\000\000\000\000\000\000\000\002\000\000\000\000\000\000\000\002\000\000\000\000\000\000""#,
            ".quad",
        ),
        ("thumbv7m-none-eabi", r#".asciz "\000\000\000\000\002\000\000\000\002\000\000""#, ".long"),
    ] {
        let a = asm(target);
        let f = body(a, "ch11_as_dyn");
        let label = f
            .iter()
            .find_map(|l| l.split_once(".Lanon").map(|(_, rest)| rest.trim_end_matches(']')))
            .map(|rest| format!(".Lanon{}", rest.split([']', ' ']).next().unwrap()))
            .unwrap_or_else(|| panic!("{target}: нет адреса таблицы в {f:?}"));
        let at = a.lines().position(|l| l == format!("{label}:")).unwrap();
        let data: Vec<String> =
            a.lines().skip(at + 1).take(3).map(|l| l.trim().replace('\t', " ")).collect();
        assert_eq!(data[0], words, "{target}: drop, size, align");
        assert_eq!(data[1], format!("{ptr} ch11_read_thermo"), "{target}: read");
        assert!(data[2].starts_with(ptr) && data[2].ends_with("4name"), "{target}: {}", data[2]);
    }
}
