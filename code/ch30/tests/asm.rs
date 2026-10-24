//! Ассемблер, который глава приводит в тексте: release-сборка x86-64
//! закреплённым cargo, одна единица кодогенерации, синтаксис Intel.
//!
//! - 30.1: `sum_to` — замкнутая формула, выброшенный результат — пустой
//!   замер, константа — `movabs` готового ответа.
//! - 30.3: побитовую CRC LLVM заменил таблицей.
//! - 30.5: лист из чужого крейта встроен, не лист — переход в чужой
//!   крейт, `#[inline]` и обобщённая — встроены, двойники слиты; с LTO
//!   встроено и чужое; `#[cold]` и `cold_path` уводят ветку из горячего
//!   пути; target-cpu с AVX-512 — `vpmullq`.
//! - 30.6: проверка вынесена из цикла суммы, осталась в гистограмме по
//!   срезу и пропала в гистограмме по массиву; сумма с проверкой
//!   переполнения не векторизована, сумма в `u64` — векторизована; флаг
//!   `RefCell` проверен один раз до цикла, а с вызовом внутри
//!   заимствования пишется на каждом шаге; отбор с ветвлением —
//!   условный переход по данным, без ветвления — сдвиг и сложение.
//!
//! Регистры и раскладка — наблюдение для rustc 1.97, не гарантия. В задании
//! CI со случайной раскладкой тесты пропускаются по префиксу `rustc_`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Сборка ассемблера: `lib` — библиотека в release, `native` — она же с
/// `-C target-cpu=emeraldrapids` (процессор машины, где снят текст;
/// задан явно, чтобы тест не зависел от машины), `lto` — программа
/// `inlining` в профиле release-lto.
fn asm(kind: &'static str) -> &'static str {
    static CACHE: OnceLock<Mutex<HashMap<&str, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut map = cache.lock().unwrap();
    map.entry(kind).or_insert_with(|| Box::leak(build(kind).into_boxed_str()))
}

fn build(kind: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch30-asm-{kind}"));
    let mut cmd = Command::new("cargo");
    cmd.current_dir(dir).env_remove("RUSTUP_TOOLCHAIN").env_remove("RUSTFLAGS").arg("rustc");
    let (profile, prefix) = match kind {
        "lto" => {
            cmd.args(["--profile", "release-lto", "--bin", "inlining"]);
            ("release-lto", "inlining-")
        }
        _ => {
            cmd.args(["--release", "--lib"]);
            ("release", "ch30-")
        }
    };
    cmd.arg("--target-dir").arg(&out_dir);
    cmd.args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"]);
    cmd.args(["-C", "llvm-args=-x86-asm-syntax=intel"]);
    if kind == "native" {
        cmd.args(["-C", "target-cpu=emeraldrapids"]);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let deps = out_dir.join(profile).join("deps");
    let file = std::fs::read_dir(&deps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with(prefix) && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Строки функции, метка которой кончается на `name:` (имя v0 с длиной,
/// например `6bounds10hist_slice`): инструкции и метки блоков, без
/// директив и комментариев, табуляции заменены пробелами.
fn body(asm: &str, name: &str) -> Vec<String> {
    let label = format!("{name}:");
    let start = asm
        .lines()
        .position(|l| l.ends_with(&label) && l.starts_with("_R"))
        .unwrap_or_else(|| panic!("нет функции {name}"));
    asm.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with(".Lfunc_end"))
        .map(|l| l.trim().replace('\t', " "))
        .filter(|l| {
            !l.is_empty() && !l.starts_with('#') && (!l.starts_with('.') || l.ends_with(':'))
        })
        .collect()
}

/// Циклы функции: от метки до перехода назад на неё, включительно.
fn loops(body: &[String]) -> Vec<&[String]> {
    let mut out = Vec::new();
    for (j, l) in body.iter().enumerate() {
        let Some(target) = l.split_whitespace().nth(1).filter(|_| l.starts_with('j')) else {
            continue;
        };
        if let Some(i) = body[..j].iter().position(|m| *m == format!("{target}:")) {
            out.push(&body[i..=j]);
        }
    }
    out
}

fn has(body: &[String], insn: &str) -> bool {
    body.iter().any(|l| l == insn)
}

fn count(body: &[String], prefix: &str) -> usize {
    body.iter().filter(|l| l.starts_with(prefix)).count()
}

fn mentions(body: &[String], what: &str) -> bool {
    body.iter().any(|l| l.contains(what))
}

#[test]
fn rustc_pitfalls() {
    let a = asm("lib");
    let sum_to = body(a, "8pitfalls6sum_to");
    assert!(loops(&sum_to).is_empty() && has(&sum_to, "mul rcx"), "{sum_to:?}");
    let discarded = body(a, "8pitfalls14time_discarded");
    assert!(loops(&discarded).is_empty(), "{discarded:?}");
    assert_eq!(count(&discarded, "call"), 2, "только now и elapsed: {discarded:?}");
    let constant = body(a, "8pitfalls13time_constant");
    assert!(has(&constant, "movabs rax, 499999500000"), "{constant:?}");
    let work = body(a, "8pitfalls11sum_squares");
    assert!(count(&work, "pmuludq") >= 2, "{work:?}");
}

#[test]
fn rustc_crc_bitwise_became_a_table() {
    let crc = body(asm("lib"), "3hot11crc_bitwise");
    // Загрузка u16 из таблицы по индексу-байту, два байта за оборот.
    let lookups = crc
        .iter()
        .filter(|l| l.starts_with("movzx") && l.contains("word ptr [rdx + 2*"))
        .count()
        + crc.iter().filter(|l| l.starts_with("xor") && l.contains("word ptr [rdx + 2*")).count();
    assert!(lookups >= 2, "{crc:?}");
    let inner = loops(&crc);
    assert_eq!(inner.len(), 1, "восьми шагов на байт нет: {crc:?}");
}

#[test]
fn rustc_cross_crate_inlining() {
    let a = asm("lib");
    assert_eq!(body(a, "8inlining10call_scale"), ["lea eax, [rdi + 2*rdi]", "inc eax", "ret"]);
    let sum = body(a, "8inlining8call_sum");
    assert_eq!(sum.len(), 1, "{sum:?}");
    assert!(
        sum[0].starts_with("jmp qword ptr") && sum[0].contains("8ch30_dep10scaled_sum"),
        "{sum:?}"
    );
    let inline = body(a, "8inlining15call_sum_inline");
    assert!(count(&inline, "paddd") >= 4 && !mentions(&inline, "ch30_dep"), "{inline:?}");
    // Обобщённая конкретизировалась в то же тело — и стала его псевдонимом.
    assert!(
        a.lines().any(|l| l.contains("8inlining16call_sum_generic = ")
            && l.ends_with("8inlining15call_sum_inline")),
        "нет псевдонима"
    );
}

#[test]
fn rustc_lto_inlines_the_other_crate() {
    let sum = body(asm("lto"), "8inlining8call_sum");
    assert!(count(&sum, "paddd") >= 4 && !mentions(&sum, "scaled_sum"), "{sum:?}");
}

#[test]
fn rustc_cold_paths() {
    let a = asm("lib");
    let plain = body(a, "8inlining11check_plain");
    assert_eq!(&plain[..2], ["cmp edi, 1000", plain[1].as_str()]);
    assert!(plain[1].starts_with("jbe "), "без подсказки отчёт идёт первым: {plain:?}");
    for f in ["8inlining10check_cold", "8inlining10check_hint"] {
        let b = body(a, f);
        assert_eq!(b[0], "cmp edi, 1000");
        assert!(b[1].starts_with("ja "), "{f}: {b:?}");
        assert!(
            b[2].starts_with("lea eax")
                && b.iter().position(|l| l == "ret") < b.iter().position(|l| l.starts_with("call")),
            "{f}: {b:?}"
        );
    }
}

#[test]
fn rustc_native_avx512() {
    let b = body(asm("native"), "8pitfalls11sum_squares");
    assert!(count(&b, "vpmullq ymm") >= 4, "{b:?}");
    assert!(!mentions(&b, "zmm"), "по умолчанию 256 бит: {b:?}");
}

#[test]
fn rustc_bounds_checks() {
    let a = asm("lib");
    let indexed = body(a, "6bounds11sum_indexed");
    // Проверка одна, до цикла: n − 1 < len.
    assert!(mentions(&indexed, "panic_bounds_check"), "{indexed:?}");
    assert!(loops(&indexed).iter().all(|l| !mentions(l, "panic")), "{indexed:?}");
    assert!(loops(&indexed).iter().any(|l| count(l, "paddd") >= 2), "{indexed:?}");
    let slice = body(a, "6bounds10hist_slice");
    let ls = loops(&slice);
    assert_eq!(ls.len(), 1, "{slice:?}");
    assert_eq!(count(ls[0], "inc dword ptr"), 1, "не развёрнут: {slice:?}");
    assert!(count(ls[0], "jbe ") == 1 && mentions(&slice, "panic_bounds_check"), "{slice:?}");
    let array = body(a, "6bounds10hist_array");
    assert!(!mentions(&array, "panic"), "{array:?}");
    assert!(loops(&array).iter().any(|l| count(l, "inc dword ptr") == 4), "{array:?}");
    let scale = body(a, "6bounds10scale_into");
    assert!(count(&scale, "cmov") >= 2, "граница векторной части — минимум длин: {scale:?}");
    assert!(loops(&scale).iter().any(|l| count(l, "movdqu xmmword ptr [rdx") == 2), "{scale:?}");
}

#[test]
fn rustc_overflow_and_refcell() {
    let a = asm("lib");
    let wrapping = body(a, "6checks12sum_wrapping");
    assert!(loops(&wrapping).iter().any(|l| count(l, "paddd") == 2), "{wrapping:?}");
    let strict = body(a, "6checks10sum_strict");
    let ls = loops(&strict);
    assert_eq!(ls.len(), 1, "{strict:?}");
    assert!(count(ls[0], "jb ") == 1 && !mentions(ls[0], "xmm"), "{strict:?}");
    assert!(mentions(&strict, "overflow_panic"), "{strict:?}");
    let wide = body(a, "6checks8sum_wide");
    assert!(loops(&wide).iter().any(|l| count(l, "paddq") == 2), "{wide:?}");
    // Флаг заимствования — первое поле RefCell, по адресу из rdx.
    for f in ["6checks12hist_refcell", "6checks17hist_refcell_once"] {
        let b = body(a, f);
        assert_eq!(count(&b, "cmp qword ptr [rdx], 0"), 1, "{f}: {b:?}");
        assert!(loops(&b).iter().all(|l| !mentions(l, "qword ptr [rdx]")), "{f}: {b:?}");
        assert!(loops(&b).iter().any(|l| count(l, "inc dword ptr") == 4), "{f}: {b:?}");
    }
    let call = body(a, "6checks17hist_refcell_call");
    let flag_set =
        |l: &[String]| l.iter().any(|x| x.starts_with("mov qword ptr [") && x.ends_with("], -1"));
    assert!(loops(&call).iter().any(|l| flag_set(l) && mentions(l, "call ")), "{call:?}");
    let cell = body(a, "6checks14hist_cell_call");
    assert!(!mentions(&cell, "qword ptr"), "{cell:?}");
}

#[test]
fn rustc_branches() {
    let a = asm("lib");
    let branchy = body(a, "8branches14select_branchy");
    let data_jump = |b: &[String]| b.iter().any(|l| l.starts_with("jns ") || l.starts_with("js "));
    assert!(data_jump(&branchy), "{branchy:?}");
    let branchless = body(a, "8branches17select_branchless");
    assert!(!data_jump(&branchless), "{branchless:?}");
    let ls = loops(&branchless);
    assert!(ls.iter().any(|l| count(l, "shr") == 1 && count(l, "cmovb") == 1), "{branchless:?}");
    let count_big = body(a, "8branches9count_big");
    assert!(!data_jump(&count_big) && count(&count_big, "psrlw") >= 2, "{count_big:?}");
}
