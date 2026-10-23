//! Разделы 14.3–14.6 и 14.8: ассемблер, который глава приводит в тексте,
//! снятый с release-сборки закреплённым cargo для x86-64 и трёх целей
//! Cortex-M.
//!
//! - `Rc::clone` — обычный инкремент и проверка переполнения, `Arc::clone`
//!   — `lock inc` на x86-64 и цикл `ldrex`/`strex` на Cortex-M3 без
//!   барьеров; `drop` у `Arc` — с барьерами `dmb` до и после.
//! - На Cortex-M0 `Arc` нет вовсе.
//! - Через `&i32` значение читается один раз, через `&Cell<i32>` — снова
//!   после вызова.
//! - `RefCell::borrow_mut` — сравнение флага с нулём и ветка паники.
//! - `LazyLock` проверяется один раз до цикла.
//! - `Rc<Vec<_>>` — на один переход по указателю больше, чем `Rc<[_]>`.
//!
//! Регистры и раскладка — наблюдение для rustc 1.97, не гарантия. В задании
//! CI со случайной раскладкой (там только nightly) тесты пропускаются по
//! префиксу `rustc_`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Ассемблер библиотеки для цели `target`, собранный один раз на прогон.
fn asm(target: &'static str) -> &'static str {
    static CACHE: OnceLock<Mutex<HashMap<&str, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut map = cache.lock().unwrap();
    map.entry(target).or_insert_with(|| Box::leak(build(target).into_boxed_str()))
}

fn build(target: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch14-asm-{target}"));
    let mut cmd = Command::new("cargo");
    cmd.current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--release", "--lib", "--target", target, "--target-dir"])
        .arg(&out_dir);
    if target.starts_with("thumb") {
        cmd.args(["--no-default-features", "--features", "alloc"]);
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
            n.starts_with("ch14-") && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции, метка которой содержит все `parts`: без директив,
/// меток и комментариев, пробелы и табуляции сжаты до одного пробела. Для
/// функций с `no_mangle` достаточно имени.
fn body(asm: &str, parts: &[&str]) -> Vec<String> {
    let start = asm
        .lines()
        .position(|l| {
            l.ends_with(':')
                && !l.starts_with('.')
                && (parts.len() > 1 || l == format!("{}:", parts[0]))
                && parts.iter().all(|p| l.contains(p))
        })
        .unwrap_or_else(|| panic!("нет функции {parts:?}"));
    asm.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with(".Lfunc_end"))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
        .collect()
}

fn has(body: &[String], insn: &str) -> bool {
    body.iter().any(|l| l == insn)
}

/// Сколько инструкций начинается с `prefix`.
fn count(body: &[String], prefix: &str) -> usize {
    body.iter().filter(|l| l.starts_with(prefix)).count()
}

const X86: &str = "x86_64-unknown-linux-gnu";
const M0: &str = "thumbv6m-none-eabi";
const M3: &str = "thumbv7m-none-eabi";
const M4F: &str = "thumbv7em-none-eabihf";

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_rc_and_arc_clone() {
    let rc = body(asm(X86), &["ch14_rc_clone"]);
    assert!(has(&rc, "inc qword ptr [rax]") && has(&rc, "ud2"), "{rc:?}");
    assert!(!rc.iter().any(|l| l.contains("lock")), "{rc:?}");
    let arc = body(asm(X86), &["ch14_arc_clone"]);
    assert!(has(&arc, "lock inc qword ptr [rax]") && has(&arc, "ud2"), "{arc:?}");
    assert!(arc.iter().any(|l| l.starts_with("jle")), "{arc:?}");
    let drop = body(asm(X86), &["ch14_arc_drop"]);
    assert!(has(&drop, "lock dec qword ptr [rdi]"), "{drop:?}");
    assert!(!drop.iter().any(|l| l.contains("fence")), "барьер Acquire на x86 не нужен: {drop:?}");
    for t in [M3, M4F] {
        let rc = body(asm(t), &["ch14_rc_clone"]);
        assert!(!rc.iter().any(|l| l.starts_with("ldrex")), "{rc:?}");
        let arc = body(asm(t), &["ch14_arc_clone"]);
        assert!(has(&arc, "ldrex r1, [r0]") && has(&arc, "strex r3, r2, [r0]"), "{arc:?}");
        assert_eq!(count(&arc, "dmb"), 0, "Relaxed: {arc:?}");
        let drop = body(asm(t), &["ch14_arc_drop"]);
        assert_eq!(count(&drop, "dmb sy"), 2, "Release и Acquire: {drop:?}");
    }
    assert!(!asm(M0).contains("ch14_arc_clone:"), "на Cortex-M0 Arc нет");
    assert!(asm(M0).contains("ch14_rc_clone:"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_shared_ref_read_once() {
    for (t, load) in [(X86, "dword ptr [rdi]"), (M3, "[r0]"), (M0, "[r0]")] {
        let r = body(asm(t), &["ch14_ref_twice"]);
        let c = body(asm(t), &["ch14_cell_twice"]);
        let loads =
            |b: &[String]| b.iter().filter(|l| l.contains(load) && !l.starts_with("str")).count();
        assert_eq!(loads(&r), 1, "{t}: {r:?}");
        let reload = if t == X86 { "dword ptr [rbx]" } else { "[r4]" };
        assert_eq!(loads(&c) + c.iter().filter(|l| l.contains(reload)).count(), 2, "{t}: {c:?}");
    }
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_refcell_flag() {
    let inc = body(asm(X86), &["ch14_refcell_inc"]);
    assert_eq!(inc[0], "cmp qword ptr [rdi], 0", "{inc:?}");
    assert!(has(&inc, "inc dword ptr [rdi + 8]"), "{inc:?}");
    assert!(inc.iter().any(|l| l.contains("panic_already_borrowed")), "{inc:?}");
    let get = body(asm(X86), &["ch14_refcell_get"]);
    assert!(get.iter().any(|l| l.contains("9223372036854775807")), "{get:?}");
    assert!(get.iter().any(|l| l.contains("panic_already_mutably_borrowed")), "{get:?}");
    let cell = body(asm(X86), &["ch14_cell_inc"]);
    assert_eq!(cell, ["inc dword ptr [rdi]", "ret"]);
    let m3 = body(asm(M3), &["ch14_refcell_inc"]);
    assert!(has(&m3, "ldreq r1, [r0, #4]"), "{m3:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_lazy_checked_once() {
    let b = body(asm(X86), &["ch14_lazy_crc"]);
    assert!(has(&b, "mov eax, dword ptr [rbx + 512]"), "состояние Once за таблицей: {b:?}");
    assert_eq!(b.iter().filter(|l| l.contains("4Once4call")).count(), 1, "{b:?}");
    let first_loop = b.iter().position(|l| l.contains("byte ptr [rdi + rcx]")).unwrap();
    let check = b.iter().position(|l| l.contains("[rbx + 512]")).unwrap();
    assert!(check < first_loop);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_rc_vec_one_more_hop() {
    let v = body(asm(X86), &["ch14_get_rc_vec"]);
    assert!(
        has(&v, "mov rcx, qword ptr [rdi]") && has(&v, "mov rax, qword ptr [rcx + 24]"),
        "{v:?}"
    );
    let s = body(asm(X86), &["ch14_get_rc_slice"]);
    assert!(has(&s, "mov rax, qword ptr [rdi + 8]"), "длина в самом указателе: {s:?}");
    assert!(has(&s, "movzx eax, word ptr [rax + 2*rsi + 16]"), "отсчёты за счётчиками: {s:?}");
}
