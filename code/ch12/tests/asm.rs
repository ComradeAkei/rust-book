//! Разделы 12.3 и 12.5: ассемблер, который глава приводит в тексте, снятый
//! с release-сборки закреплённым cargo для x86-64 и Cortex-M3.
//!
//! - Копия обобщённой функции получает окружение замыкания первым
//!   аргументом: ничего для замыкания без захвата, адрес для захвата по
//!   ссылке, само число для захвата по значению.
//! - Обобщённая калибровка векторизована и не содержит вызовов, а чтение
//!   захваченных `k` и `b` вынесено из цикла.
//! - Через `&mut dyn FnMut` адрес `call_mut` читается из слова 4 таблицы
//!   один раз, а вызов по нему остаётся в каждой итерации.
//! - `Box<dyn FnOnce>` вызывается через слово 3 таблицы, затем память
//!   освобождается по размеру и выравниванию из слов 1 и 2.
//! - Таблица обработчиков — переход по индексу, трамплин для `qsort_r`
//!   встроил замыкание.
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
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch12-asm-{target}"));
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
            n.starts_with("ch12-") && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции, метка которой содержит все `parts`: без директив,
/// меток и комментариев, табуляции заменены пробелами. Для функций с
/// `no_mangle` достаточно имени, для копий обобщённых функций — частей
/// искажённого имени v0, например `11apply_twice` и `ch12_twice_ref`.
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
        .map(|l| l.trim().replace('\t', " "))
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
        .collect()
}

fn has(body: &[String], insn: &str) -> bool {
    body.iter().any(|l| l == insn)
}

/// Вызовы: `call` на x86-64, `bl` и `blx` на Cortex-M (но не `blo`).
fn calls(body: &[String]) -> Vec<&String> {
    body.iter()
        .filter(|l| l.starts_with("call") || l.starts_with("bl ") || l.starts_with("blx"))
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_environment_x86() {
    let a = asm("x86_64-unknown-linux-gnu");
    let twice = |c| body(a, &["11apply_twice", c]);
    assert_eq!(twice("ch12_twice_zst"), ["lea eax, [rdi + 8*rdi]", "ret"]);
    assert_eq!(
        twice("ch12_twice_ref"),
        ["mov eax, dword ptr [rdi]", "imul esi, eax", "imul eax, esi", "ret"]
    );
    assert_eq!(twice("ch12_twice_val"), ["mov eax, edi", "imul eax, edi", "imul eax, esi", "ret"]);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_calibrate_x86() {
    let a = asm("x86_64-unknown-linux-gnu");
    let generic = body(a, &["ch12_scale_generic"]);
    assert!(calls(&generic).is_empty(), "{generic:?}");
    assert!(has(&generic, "pmullw xmm2, xmm0") && has(&generic, "paddw xmm2, xmm1"), "{generic:?}");
    // k и b читаются один раз, до цикла.
    assert_eq!(generic.iter().filter(|l| l.contains("word ptr [rdx")).count(), 2, "{generic:?}");
    let dynamic = body(a, &["ch12_map_dyn"]);
    assert!(has(&dynamic, "mov r12, qword ptr [rcx + 32]"), "{dynamic:?}");
    assert_eq!(calls(&dynamic), ["call r12"]);
    let closure = body(a, &["_RNCNv", "14ch12_scale_dyn0"]);
    assert!(has(&closure, "mov rcx, qword ptr [rdi]"), "{closure:?}");
    assert!(has(&closure, "imul ax, word ptr [rcx]"), "{closure:?}");
    assert_eq!(calls(&body(a, &["ch12_map_fn"])), ["call rbx"]);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_boxed_once_x86() {
    let a = asm("x86_64-unknown-linux-gnu");
    let once = body(a, &["ch12_call_boxed"]);
    assert_eq!(calls(&once)[0], "call qword ptr [rsi + 24]", "{once:?}");
    assert!(
        has(&once, "mov rsi, qword ptr [r14 + 8]") && has(&once, "mov rdx, qword ptr [r14 + 16]")
    );
    assert!(once.iter().any(|l| l.contains("__rust_dealloc")), "{once:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_fn_pointers_x86() {
    let a = asm("x86_64-unknown-linux-gnu");
    let d = body(a, &["ch12_dispatch"]);
    assert!(has(&d, "cmp rdi, 3") && has(&d, "jmp qword ptr [rcx + 8*rax]"), "{d:?}");
    let tramp = body(a, &["5tramp", "sort_desc"]);
    assert!(calls(&tramp).is_empty(), "замыкание не встроилось в трамплин: {tramp:?}");
    assert!(has(&tramp, "inc qword ptr [rcx]"), "{tramp:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_thumb() {
    let a = asm("thumbv7m-none-eabi");
    let generic = body(a, &["ch12_scale_generic"]);
    assert!(calls(&generic).is_empty(), "{generic:?}");
    assert!(has(&generic, "mla r3, r3, r2, r4"), "{generic:?}");
    let dynamic = body(a, &["ch12_map_dyn"]);
    assert!(has(&dynamic, "ldr r4, [r3, #16]") && has(&dynamic, "blx r4"), "{dynamic:?}");
    let d = body(a, &["ch12_dispatch"]);
    assert!(has(&d, "ldr.w r2, [r2, r0, lsl #2]") && has(&d, "bx r2"), "{d:?}");
}
