//! Раздел 0.5: clippy находит в `lints/src/lib.rs` три ошибки, которые
//! пропускает компилятор. Тест запускает clippy так же, как читатель, и
//! проверяет имена найденных линтов.

use std::path::Path;
use std::process::Command;

#[test]
#[cfg_attr(miri, ignore = "Miri не запускает внешние процессы")]
fn clippy_finds_all_three() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("lints");
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch00-lints");
    // Внешний cargo передаёт путь к себе в CARGO, так что версия совпадёт.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["clippy", "--quiet", "--message-format=json", "--target-dir"])
        .arg(&target)
        .current_dir(&dir)
        .output()
        .expect("не удалось запустить cargo clippy");
    let json = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Два линта из группы correctness запрещены по умолчанию: проверка падает.
    assert!(!out.status.success(), "clippy не нашёл ошибок:\n{stderr}");
    for lint in [
        "clippy::manual_memcpy",
        "clippy::not_unsafe_ptr_arg_deref",
        "clippy::absurd_extreme_comparisons",
    ] {
        assert!(json.contains(&format!("\"{lint}\"")), "нет {lint}:\n{stderr}");
    }
}
