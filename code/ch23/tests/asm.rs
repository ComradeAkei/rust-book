//! Раздел 23.6: раскрытый макрос не дороже рукописного кода. Функции
//! `ch23_macro_send` (регистры из `mmio!`) и `ch23_hand_send` (те же
//! `read_volatile`/`write_volatile` руками) в release-сборке под
//! thumbv7m-none-eabi дают одни и те же инструкции — настолько, что LLVM
//! сливает их в одну: `ch23_macro_send = ch23_hand_send`, псевдоним, как
//! typestate и голые записи в главе 10. Снято закреплённым
//! `cargo rustc --emit asm`; под Miri тест пропускается, в задании CI со
//! случайной раскладкой — по префиксу `rustc_`.

use std::path::PathBuf;
use std::process::Command;

fn thumb_asm() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch23-asm");
    let t = "thumbv7m-none-eabi";
    let res = Command::new("cargo")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--lib", "--release", "--no-default-features", "--target", t])
        .arg("--target-dir")
        .arg(&out)
        .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let d = out.join(t).join("release/deps");
    let file = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("ch23-") && n.ends_with(".s")
        })
        .unwrap_or_else(|| panic!("нет файла .s в {}", d.display()));
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции: строки между её меткой и `.Lfunc_end`, без
/// директив; номера локальных меток заменены, они у функций разные.
fn body(asm: &str, name: &str) -> Vec<String> {
    let start = asm.find(&format!("\n{name}:")).unwrap_or_else(|| panic!("нет {name}"));
    let rest = &asm[start + name.len() + 2..];
    let end = rest.find(".Lfunc_end").unwrap();
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '@']))
        .map(|l| {
            l.split(|c: char| c.is_whitespace() || c == ',')
                .filter(|w| !w.is_empty())
                .map(|w| if w.starts_with(".LBB") { ".LBB" } else { w })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_macro_is_free() {
    let asm = thumb_asm();
    let by_hand = body(&asm, "ch23_hand_send");
    // Обращения volatile на месте: чтения FR в цикле и запись DR.
    assert!(by_hand.iter().any(|l| l.starts_with("ldr")), "{by_hand:?}");
    assert!(by_hand.iter().any(|l| l.starts_with("str")), "{by_hand:?}");
    let alias = asm.lines().any(|l| {
        let l = l.trim();
        l == "ch23_macro_send = ch23_hand_send" || l == "ch23_hand_send = ch23_macro_send"
    });
    // Либо псевдоним (rustc 1.97 сливает одинаковые функции), либо
    // тело — инструкция в инструкцию.
    if !alias {
        assert_eq!(body(&asm, "ch23_macro_send"), by_hand);
    }
    assert!(alias, "rustc 1.97 сливал функции в псевдоним; теперь тела равны, но раздельны");
}
