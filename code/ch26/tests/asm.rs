//! Раздел 26.5: вызовы `memcpy`, `memset`, `memcmp` и `strlen`, которые
//! вставляет компилятор. Функции `src/copies.rs` собираются в release под
//! x86-64 (синтаксис Intel, как в книге), Cortex-M3 и Cortex-M0.
//!
//! Снято закреплённым `cargo rustc --emit asm`; под Miri тесты
//! пропускаются, в задании CI со случайной раскладкой — по префиксу
//! `rustc_`.

use std::path::PathBuf;
use std::process::Command;

/// Ассемблер библиотеки ch26 под `target` в release. Каждый вызов — свой
/// каталог сборки `dir`.
fn asm_of(target: &str, dir: &str, extra: &[&str]) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir);
    let res = Command::new("cargo")
        .current_dir(manifest)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args([
            "rustc",
            "-p",
            "ch26",
            "--lib",
            "--release",
            "--no-default-features",
            "--color",
            "never",
        ])
        .args(["--target", target, "--target-dir"])
        .arg(&out)
        .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
        .args(extra)
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let d = out.join(target).join("release/deps");
    // Самый свежий: от прежних сборок с другим хешем остаются старые .s.
    let file = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("ch26-") && n.ends_with(".s")
        })
        .max_by_key(|p| p.metadata().unwrap().modified().unwrap())
        .unwrap_or_else(|| panic!("нет файла .s в {}", d.display()));
    std::fs::read_to_string(file).unwrap()
}

/// Мнемоники инструкций функции: строки между её меткой и `.Lfunc_end`,
/// без директив и меток.
fn mnemonics(asm: &str, name: &str) -> Vec<String> {
    let label = format!("\n{name}:\n");
    let start = asm.find(&label).unwrap_or_else(|| panic!("нет {name}"));
    let rest = &asm[start + label.len()..];
    let end = rest.find(".Lfunc_end").unwrap();
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '@', '#']))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// Вызовы и переходы на внешние функции: `bl x`, `b x`, `call …`, `jmp …`
/// без переходов на локальные метки.
fn calls(asm: &str, name: &str) -> Vec<String> {
    mnemonics(asm, name)
        .into_iter()
        .filter(|l| {
            let mut w = l.split(' ');
            let op = w.next().unwrap();
            let arg = w.next().unwrap_or("");
            matches!(op, "bl" | "b" | "b.w" | "call" | "jmp") && !arg.starts_with(".L")
        })
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_cortex_m3_calls_aeabi_helpers() {
    let asm = asm_of("thumbv7m-none-eabi", "ch26-m3", &[]);
    assert_eq!(calls(&asm, "ch26_copy_frame"), ["bl __aeabi_memcpy"]);
    assert_eq!(calls(&asm, "ch26_store"), ["bl __aeabi_memcpy4"]);
    assert_eq!(calls(&asm, "ch26_clear"), ["bl __aeabi_memclr4"]);
    assert_eq!(calls(&asm, "ch26_fill"), ["bl __aeabi_memset"]);
    assert_eq!(calls(&asm, "ch26_same"), ["bl memcmp"]);
    assert_eq!(calls(&asm, "ch26_c_len"), ["b strlen"]);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_cortex_m0_calls_the_same() {
    let asm = asm_of("thumbv6m-none-eabi", "ch26-m0", &[]);
    assert_eq!(calls(&asm, "ch26_copy_frame"), ["bl __aeabi_memcpy"]);
    assert_eq!(calls(&asm, "ch26_store"), ["bl __aeabi_memcpy4"]);
    assert_eq!(calls(&asm, "ch26_clear"), ["bl __aeabi_memclr4"]);
    assert_eq!(calls(&asm, "ch26_fill"), ["bl __aeabi_memset"]);
    assert_eq!(calls(&asm, "ch26_same"), ["bl memcmp"]);
    assert_eq!(calls(&asm, "ch26_c_len"), ["bl strlen"]);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_x86_64_calls_libc() {
    let asm =
        asm_of("x86_64-unknown-linux-gnu", "ch26-x86", &["-C", "llvm-args=-x86-asm-syntax=intel"]);
    assert_eq!(calls(&asm, "ch26_copy_frame"), ["call qword ptr [rip + memcpy@GOTPCREL]"]);
    assert_eq!(calls(&asm, "ch26_store"), ["jmp qword ptr [rip + memcpy@GOTPCREL]"]);
    assert_eq!(calls(&asm, "ch26_fill"), ["jmp qword ptr [rip + memset@GOTPCREL]"]);
    assert_eq!(calls(&asm, "ch26_c_len"), ["jmp qword ptr [rip + strlen@GOTPCREL]"]);
    // 256 байт нулей и сравнение 32 байт — векторами, без вызова.
    assert_eq!(calls(&asm, "ch26_clear"), Vec::<String>::new());
    assert_eq!(calls(&asm, "ch26_same"), Vec::<String>::new());
}
