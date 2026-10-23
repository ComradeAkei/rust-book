//! Раздел 13.5 и 13.1: ассемблер, который глава приводит в тексте, снятый
//! с release-сборки закреплённым cargo для x86-64 и трёх целей Cortex-M.
//!
//! - Шаг итератора по срезу — сравнение указателя с концом и сдвиг.
//! - Конвейер `filter`, `map`, `fold` — один цикл без вызовов; деление на
//!   4095 заменено умножением везде, кроме Cortex-M0, где на каждый отсчёт
//!   вызывается `__aeabi_uidiv`.
//! - `for` по `chain` проверяет на каждом шаге, какая половина, и не
//!   векторизуется; `fold` по той же цепочке — два векторных цикла.
//! - Сведение по индексам на x86-64 векторизовано, но проверки границ
//!   остались в скалярном хвосте; на Cortex-M3 они в каждой итерации, а
//!   через `zip` цикл развёрнут вчетверо и проверок нет.
//! - AVX2 даёт `vpaddsw` на `ymm`; скалярное произведение `i16` — это
//!   `pmaddwd` на x86-64 и `smlad` на Cortex-M4F.
//! - `chunks_exact(16)` убирает внутренний цикл, и код вдвое короче.
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
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch13-asm-{target}"));
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
            n.starts_with("ch13-") && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции, метка которой содержит все `parts`: без директив,
/// меток и комментариев, табуляции заменены пробелами. Для функций с
/// `no_mangle` достаточно имени.
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
fn rustc_slice_next() {
    let next = body(asm(X86), &["ch13_next"]);
    assert!(has(&next, "cmp rax, qword ptr [rdi + 8]"), "{next:?}");
    assert!(has(&next, "lea rcx, [rax + 4]") && has(&next, "mov qword ptr [rdi], rcx"), "{next:?}");
    assert!(calls(&next).is_empty());
    let next = body(asm(M3), &["ch13_next"]);
    assert!(has(&next, "ldrd r0, r2, [r0]") && has(&next, "cmp r0, r2"), "{next:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_pipeline() {
    let x86 = body(asm(X86), &["ch13_mean_mv"]);
    assert!(calls(&x86).is_empty(), "{x86:?}");
    // Деление на 4095 — умножение на 268501009 и сдвиг на 40.
    assert!(x86.iter().any(|l| l.ends_with(", 268501009")), "{x86:?}");
    assert_eq!(count(&x86, "div "), 1, "делится только сумма: {x86:?}");
    assert_eq!(count(&x86, "imul") % 2, 0);
    let m3 = body(asm(M3), &["ch13_mean_mv"]);
    assert!(calls(&m3).is_empty(), "{m3:?}");
    assert!(count(&m3, "umull") >= 4 && count(&m3, "udiv") == 1, "{m3:?}");
    let m0 = body(asm(M0), &["ch13_mean_mv"]);
    assert!(calls(&m0).iter().all(|c| c.as_str() == "bl __aeabi_uidiv"), "{m0:?}");
    assert!(calls(&m0).len() >= 3, "{m0:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_chain() {
    let a = asm(X86);
    let for_loop = body(a, &["ch13_chain_for"]);
    assert!(!for_loop.iter().any(|l| l.contains("xmm")), "{for_loop:?}");
    assert!(has(&for_loop, "test rdi, rdi"), "проверка «первая половина кончилась»: {for_loop:?}");
    let fold = body(a, &["ch13_chain_fold"]);
    assert!(count(&fold, "paddd") >= 4 && count(&fold, "movdqu") >= 4, "{fold:?}");
    let m3_for = body(asm(M3), &["ch13_chain_for"]);
    let m3_fold = body(asm(M3), &["ch13_chain_fold"]);
    assert!(count(&m3_for, "cmpne") >= 4, "{m3_for:?}");
    assert_eq!(count(&m3_fold, "cmpne"), 0, "{m3_fold:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_bounds_x86() {
    let a = asm(X86);
    let index = body(a, &["ch13_mix_index"]);
    assert!(count(&index, "paddsw") >= 2, "{index:?}");
    let c = calls(&index);
    assert_eq!(c.len(), 2, "{index:?}");
    assert!(c.iter().all(|l| l.contains("panic_bounds_check")), "{c:?}");
    for f in ["ch13_mix_assert", "ch13_mix_zip"] {
        let b = body(a, &[f]);
        assert!(count(&b, "paddsw") >= 3, "{f}: {b:?}");
        assert!(!b.iter().any(|l| l.contains("panic_bounds_check")), "{f}: {b:?}");
    }
    let avx2 = body(a, &["ch13_mix_avx2"]);
    assert!(avx2.iter().any(|l| l.starts_with("vpaddsw ymm")), "{avx2:?}");
    let dot = body(a, &["ch13_dot_i16"]);
    assert!(count(&dot, "pmaddwd") >= 2 && calls(&dot).is_empty(), "{dot:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_bounds_thumb() {
    let index = body(asm(M3), &["ch13_mix_index"]);
    assert_eq!(count(&index, "ssat"), 1, "цикл не развёрнут: {index:?}");
    let c = calls(&index);
    assert!(c.len() == 2 && c.iter().all(|l| l.contains("panic_bounds_check")), "{c:?}");
    let zip = body(asm(M3), &["ch13_mix_zip"]);
    assert!(count(&zip, "ssat") >= 5 && calls(&zip).is_empty(), "{zip:?}");
    let zip = body(asm(M4F), &["ch13_mix_zip"]);
    assert!(count(&zip, "qadd16") >= 5 && count(&zip, "ssat") == 0, "{zip:?}");
    // Чтение — по одному отсчёту: qadd16 складывает только младшие половины.
    assert!(count(&zip, "ldrsh") >= 8, "{zip:?}");
    let dot = body(asm(M4F), &["ch13_dot_i16"]);
    assert!(count(&dot, "smlad") >= 2 && calls(&dot).is_empty(), "{dot:?}");
    let dot = body(asm(M3), &["ch13_dot_i16"]);
    assert!(count(&dot, "mla") >= 4 && count(&dot, "smlad") == 0, "{dot:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_chunks_exact() {
    let a = asm(X86);
    let chunks = body(a, &["ch13_peaks_chunks"]);
    let exact = body(a, &["ch13_peaks_exact"]);
    assert!(exact.len() * 2 < chunks.len(), "{} и {}", exact.len(), chunks.len());
    assert!(count(&exact, "pmaxsw") >= 10, "{exact:?}");
    // Блок из 16 отсчётов — две загрузки по 16 байт.
    assert!(exact.iter().any(|l| l.starts_with("movdqu xmm1, xmmword ptr [r8 - 16]")), "{exact:?}");
}

/// Раздел 13.2: `map` без потребителя — предупреждение `unused_must_use`
/// с подсказкой о ленивости.
#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_lazy_warning() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch13-lazy");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("lazy.rs");
    std::fs::write(&src, "pub fn f(data: &[i32]) {\n    data.iter().map(|x| x * 10);\n}\n")
        .unwrap();
    let out = Command::new("rustc")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .env_remove("RUSTUP_TOOLCHAIN")
        .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata", "-o"])
        .arg(dir.join("liblazy.rmeta"))
        .arg(&src)
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("warning: unused `Map` that must be used"), "{err}");
    assert!(err.contains("= note: iterators are lazy and do nothing unless consumed"), "{err}");
}
