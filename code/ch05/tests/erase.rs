//! Раздел 5.3: `ch05_first` и `ch05_first_static` различаются только
//! временами жизни, и после оптимизации от второй остаётся псевдоним
//! первой. Тест собирает `src/erase.rs` с `-O` и ищет псевдоним в LLVM IR.

use std::process::Command;

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn static_variant_is_alias() {
    let ir = std::env::temp_dir().join(format!("ch05-erase-{}.ll", std::process::id()));
    let out = Command::new("rustc")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["--edition", "2024", "--crate-type", "lib", "-O", "--emit", "llvm-ir", "-o"])
        .arg(&ir)
        .arg("src/erase.rs")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = std::fs::read_to_string(&ir).unwrap();
    let _ = std::fs::remove_file(&ir);
    let alias =
        text.lines().find(|l| l.starts_with("@ch05_first_static = ")).unwrap_or_else(|| {
            panic!(
                "нет определения @ch05_first_static:\n{}",
                text.chars().take(2000).collect::<String>()
            )
        });
    assert!(alias.contains(" alias ") && alias.ends_with("@ch05_first"), "{alias}");
    let has_body = |name: &str| {
        text.lines().any(|l| l.starts_with("define ") && l.contains(&format!("@{name}(")))
    };
    assert!(has_body("ch05_first"), "нет тела ch05_first");
    assert!(!has_body("ch05_first_static"), "у ch05_first_static своё тело");
}
