//! Задача D, вторая часть: почему без `#![no_builtins]` своя `memset`
//! может вызвать сама себя. Фрагмент `fragments/without_no_builtins.rs`
//! подключает тот же `src/d_mem.rs` в крейт без этого атрибута и
//! собирается закреплённым rustc под thumbv7m-none-eabi.
//!
//! Под Miri тест пропускается, в задании CI со случайной раскладкой — по
//! префиксу `rustc_`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn asm(opt: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ch26-naive-{opt}.s"));
    let res = Command::new("rustc")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .args(["--edition", "2024", "--crate-type", "lib", "--target", "thumbv7m-none-eabi"])
        .args([
            "-C",
            &format!("opt-level={opt}"),
            "-C",
            "debuginfo=0",
            "--emit",
            "asm",
            "--color",
            "never",
            "-o",
        ])
        .arg(&out)
        .arg("fragments/without_no_builtins.rs")
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    std::fs::read_to_string(out).unwrap()
}

/// Тело функции: от её метки до `.Lfunc_end`.
fn body<'a>(asm: &'a str, name: &str) -> &'a str {
    let label = format!("\n{name}:\n");
    let start = asm.find(&label).unwrap_or_else(|| panic!("нет {name}")) + label.len();
    let end = start + asm[start..].find(".Lfunc_end").unwrap();
    &asm[start..end]
}

fn calls(body: &str, target: &str) -> bool {
    body.lines().any(|l| {
        let mut w = l.split_whitespace();
        matches!(w.next(), Some("bl" | "b" | "b.w")) && w.next() == Some(target)
    })
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_d_aeabi_memset_calls_itself_without_no_builtins() {
    let a = asm("3");
    // memset внутри __aeabi_memset встроена, и её цикл LLVM узнал как
    // идиому memset — а memset под ARM EABI и есть __aeabi_memset.
    assert!(calls(body(&a, "__aeabi_memset"), "__aeabi_memset"), "{}", body(&a, "__aeabi_memset"));
    assert!(calls(body(&a, "__aeabi_memcpy"), "__aeabi_memcpy"), "{}", body(&a, "__aeabi_memcpy"));
    // В функциях с именами memset и memcpy LLVM идиом не ищет: цикл
    // остаётся циклом.
    assert!(!calls(body(&a, "memset"), "__aeabi_memset"));
    assert!(!calls(body(&a, "memcpy"), "__aeabi_memcpy"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_d_no_recursion_at_opt_level_0() {
    // Без оптимизации идиомы не ищутся: та же ошибка видна только в release.
    let a = asm("0");
    assert!(!calls(body(&a, "__aeabi_memset"), "__aeabi_memset"));
}
