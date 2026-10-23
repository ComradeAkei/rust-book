//! Разделы 27.4 и 27.5: во что превращается HAL. Функции
//! `src/probe.rs` собираются в release под Cortex-M3 (thumbv7m-none-eabi)
//! закреплённым `cargo rustc --emit asm`, без LTO — как библиотека, а не
//! прошивка. Под Miri тесты пропускаются, в задании CI со случайной
//! раскладкой — по префиксу `rustc_`.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Ассемблер библиотеки ch27-hal с возможностью `probe`; собирается один
/// раз на все тесты.
fn asm() -> &'static str {
    static ASM: OnceLock<String> = OnceLock::new();
    ASM.get_or_init(|| {
        let target = "thumbv7m-none-eabi";
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch27-hal-asm");
        let res = Command::new("cargo")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env_remove("RUSTUP_TOOLCHAIN")
            .env_remove("RUSTFLAGS")
            .args(["rustc", "-p", "ch27-hal", "--lib", "--release", "--features", "probe"])
            .args(["--color", "never", "--target", target, "--target-dir"])
            .arg(&out)
            .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
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
                n.starts_with("ch27_hal-") && n.ends_with(".s")
            })
            .max_by_key(|p| p.metadata().unwrap().modified().unwrap())
            .unwrap_or_else(|| panic!("нет файла .s в {}", d.display()));
        std::fs::read_to_string(file).unwrap()
    })
}

/// Мнемоники функции без директив и меток. Одинаковые функции LLVM
/// склеивает: `a = b` вместо тела, — тогда берётся тело `b`.
fn mnemonics(name: &str) -> Vec<String> {
    let asm = asm();
    if let Some(other) = asm.lines().find_map(|l| l.strip_prefix(&format!("{name} = "))) {
        return mnemonics(other.trim());
    }
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

fn count(code: &[String], op: &str) -> usize {
    code.iter().filter(|l| l.split(' ').next().unwrap().trim_end_matches(".w") == op).count()
}

/// Переходы на другие функции: `bl x` или `b x` не на локальную метку.
fn calls(code: &[String]) -> Vec<&String> {
    code.iter()
        .filter(|l| {
            let mut w = l.split(' ');
            let op = w.next().unwrap();
            let arg = w.next().unwrap_or("");
            matches!(op, "bl" | "b" | "b.w") && !arg.starts_with(".L")
        })
        .collect()
}

// ANCHOR: same
#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_raw_pac_hal_are_one_store() {
    let raw = mnemonics("ch27_raw_set_high");
    assert_eq!(mnemonics("ch27_pac_set_high"), raw);
    assert_eq!(mnemonics("ch27_hal_set_high"), raw);
    // Адрес 0x4002_5004 — movw и movt, значение 1, одна запись, ни
    // одного чтения.
    assert!(raw.contains(&"movw r0, #20484".to_owned()), "{raw:#?}");
    assert!(raw.contains(&"movt r0, #16386".to_owned()), "{raw:#?}");
    assert_eq!((count(&raw, "str"), count(&raw, "ldr")), (1, 0), "{raw:#?}");
}
// ANCHOR_END: same

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_raw_and_pac_are_merged() {
    // Тела совпали настолько, что LLVM оставил одно, а второе имя — его
    // псевдоним. Наблюдение, а не гарантия.
    assert!(asm().contains("ch27_raw_set_high = ch27_pac_set_high"));
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_into_output_is_three_bitband_stores() {
    let code = mnemonics("ch27_hal_into_output");
    // AFSEL, DIR, DEN — три записи в псевдонимы, без чтения регистров и
    // без вызова `bitband::alias`: адреса посчитаны при компиляции.
    assert_eq!((count(&code, "str"), count(&code, "ldr")), (3, 0), "{code:#?}");
    assert!(calls(&code).is_empty(), "{code:#?}");
    // 0x424A_8000 — псевдоним бита 0 DIR порта F.
    assert!(code.contains(&"movt r0, #16970".to_owned()), "{code:#?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_hc595_is_inlined_and_unrolled() {
    let code = mnemonics("ch27_hc595_write");
    // Восемь бит по три записи — данные, фронт и спад тактового, — и
    // два для защёлки. Вызовов нет: трейты embedded-hal исчезли.
    assert!(calls(&code).is_empty(), "{code:#?}");
    assert_eq!((count(&code, "str"), count(&code, "ldr")), (26, 0), "{code:#?}");
}
