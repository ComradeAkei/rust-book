//! Проверка кодогенерации главы 19: инструкции, которые обещают разделы,
//! действительно попадают в ассемблер. Снято с release-сборки закреплённым
//! `cargo rustc --emit asm` для x86-64 в синтаксисе Intel. Тест запускает
//! cargo — под Miri пропускается.
//!
//! - `rdtsc` и `lfence` — из `asm::rdtsc` (раздел 19.1);
//! - `imul` — из `asm::muladd` (19.1);
//! - `paddd` — сложение четырёх i32 на SSE2 из `simd::sum_sse2` (19.3);
//! - `vpaddd` — восьми i32 на AVX2 из `simd::sum_avx2` (19.3).
//!
//! `asm::rdtsc`/`asm::muladd` не вызываются внутри крейта, поэтому rustc
//! хранит их как MIR для межкрейтового инлайна и в ассемблер библиотеки их
//! инструкции не кладёт. Их снимаем с примера `emit_probe`, который эти
//! функции зовёт; `paddd`/`vpaddd` снимаем с самой библиотеки, где у
//! `sum_sse2`/`sum_avx2` настоящие векторные циклы (в инлайне под константу
//! они бы схлопнулись).

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn asm() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(build)
}

/// Снять ассемблер одной сущности крейта (`--lib` или `--example <name>`),
/// вернуть содержимое единственного `.s` из соответствующего каталога.
fn emit(out: &std::path::Path, what: &[&str], subdir: &str, prefix: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let res = Command::new("cargo")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--release", "--target", "x86_64-unknown-linux-gnu"])
        .args(what)
        .arg("--target-dir")
        .arg(out)
        .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
        .args(["-C", "llvm-args=-x86-asm-syntax=intel"])
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let d = out.join("x86_64-unknown-linux-gnu/release").join(subdir);
    let file = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with(prefix) && n.ends_with(".s")
        })
        .unwrap_or_else(|| panic!("нет файла .s в {}", d.display()));
    std::fs::read_to_string(file).unwrap()
}

fn build() -> String {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch19-asm");
    // Векторные циклы — из библиотеки; asm!-примитивы — из примера-драйвера.
    let lib = emit(&out, &["--lib"], "deps", "ch19-");
    let ex = emit(&out, &["--example", "emit_probe"], "examples", "emit_probe-");
    format!("{lib}\n{ex}")
}

/// Мнемоника присутствует как инструкция (отдельным словом), а не в имени
/// символа или комментарии.
fn has_insn(asm: &str, mnemonic: &str) -> bool {
    asm.lines().any(|l| {
        let t = l.trim();
        !t.starts_with(['.', '#', '@']) && t.split_whitespace().next() == Some(mnemonic)
    })
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn chapter_instructions_are_emitted() {
    let s = asm();
    assert!(has_insn(s, "rdtsc"), "нет rdtsc (раздел 19.1)");
    assert!(has_insn(s, "lfence"), "нет lfence-сериализации (19.1)");
    assert!(has_insn(s, "imul"), "нет imul из muladd (19.1)");
    assert!(has_insn(s, "paddd"), "нет SSE2 paddd (19.3)");
    assert!(has_insn(s, "vpaddd"), "нет AVX2 vpaddd (19.3)");
}
