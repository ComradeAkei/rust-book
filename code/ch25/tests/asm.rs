//! Раздел 25.5: таблица обратных заменяет деление. В release-сборке под
//! thumbv6m-none-eabi (Cortex-M0, деления нет) `ch25_scale_div` вызывает
//! программное деление `__aeabi_uidiv`, а `ch25_scale_recip` обходится
//! загрузкой из `RECIP`, `muls` и сдвигом — без единого вызова.
//!
//! Раздел 25.7: CRC-16 кадра по таблице и побитово под thumbv7m-none-eabi
//! при `opt-level = "s"` и при `opt-level = 3`.
//!
//! Снято закреплённым `cargo rustc --emit asm`; под Miri тесты
//! пропускаются, в задании CI со случайной раскладкой — по префиксу
//! `rustc_`.

use std::path::PathBuf;
use std::process::Command;

/// Ассемблер библиотеки `package` под `target` в release с доп.
/// флагами rustc `extra`. Каждый вызов — свой каталог сборки `dir`.
fn asm_of(package: &str, target: &str, dir: &str, extra: &[&str]) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir);
    let res = Command::new("cargo")
        .current_dir(manifest)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "-p", package, "--lib", "--release", "--color", "never"])
        .args(if package == "ch25" { &["--no-default-features"][..] } else { &[] })
        .args(["--target", target, "--target-dir"])
        .arg(&out)
        .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
        .args(extra)
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let d = out.join(target).join("release/deps");
    let prefix = format!("{}-", package.replace('-', "_"));
    // Самый свежий: от прежних сборок с другим хешем остаются старые .s.
    let file = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with(&prefix) && n.ends_with(".s")
        })
        .max_by_key(|p| p.metadata().unwrap().modified().unwrap())
        .unwrap_or_else(|| panic!("нет файла .s в {}", d.display()));
    std::fs::read_to_string(file).unwrap()
}

fn thumb_asm(target: &str) -> String {
    asm_of("ch25", target, "ch25-asm", &[])
}

/// Мнемоники инструкций функции: строки между её меткой и `.Lfunc_end`,
/// без директив и меток. `name` — имя или хвост искажённого имени.
fn mnemonics(asm: &str, name: &str) -> Vec<String> {
    let label = asm
        .lines()
        .find(|l| l.ends_with(&format!("{name}:")) && !l.starts_with(['\t', ' ', '.']))
        .unwrap_or_else(|| panic!("нет {name}"));
    let start = asm.find(&format!("\n{label}")).unwrap();
    let rest = &asm[start + label.len() + 1..];
    let end = rest.find(".Lfunc_end").unwrap();
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '@']))
        .map(|l| l.replace('\t', " "))
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_reciprocal_replaces_division_on_m0() {
    let asm = thumb_asm("thumbv6m-none-eabi");
    let div = mnemonics(&asm, "ch25_scale_div");
    assert!(div.iter().any(|l| l == "bl __aeabi_uidiv"), "{div:?}");
    let recip = mnemonics(&asm, "ch25_scale_recip");
    assert!(recip.iter().any(|l| l.starts_with("muls")), "{recip:?}");
    assert!(recip.iter().any(|l| l.starts_with("lsrs") && l.ends_with("#20")), "{recip:?}");
    assert!(!recip.iter().any(|l| l.starts_with("bl")), "{recip:?}");
    // Даже деление на константу 7: умножения 32 × 32 → 64 у M0 нет.
    let div7 = mnemonics(&asm, "ch25_scale_div7");
    assert!(div7.iter().any(|l| l == "bl __aeabi_uidiv"), "{div7:?}");
}

/// На Cortex-M3 деление есть: `udiv` вместо вызова, а деление на
/// константу — `umull`.
#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_m3_divides_in_hardware() {
    let asm = thumb_asm("thumbv7m-none-eabi");
    let div = mnemonics(&asm, "ch25_scale_div");
    assert!(div.iter().any(|l| l.starts_with("udiv")), "{div:?}");
    assert!(!div.iter().any(|l| l.starts_with("bl")), "{div:?}");
    // Деление на константу — умножение на обратное, без udiv.
    let div7 = mnemonics(&asm, "ch25_scale_div7");
    assert!(div7.iter().any(|l| l.starts_with("umull")), "{div7:?}");
    assert!(!div7.iter().any(|l| l.starts_with("udiv")), "{div7:?}");
}

/// CRC кадра под Cortex-M3: обёртки из `src/project.rs`. Функции
/// `telemetry-frame` встраиваются в них и компилируются с флагами `ch25`.
const BITWISE: &str = "ch25_crc_bitwise";
const TABLE: &str = "ch25_crc_table";

/// При `opt-level = "s"` (так собирается отладочная прошивка) побитовый
/// CRC остаётся циклом по восьми битам на байт, а табличный — семь
/// инструкций на байт с загрузкой `ldrh` из `CRC16_TABLE`.
#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_crc_table_at_opt_s() {
    let asm = asm_of("ch25", "thumbv7m-none-eabi", "ch25-crc-s", &["-C", "opt-level=s"]);
    let bit = mnemonics(&asm, BITWISE);
    assert!(bit.iter().any(|l| l == "movs r3, #8" || l.ends_with(", #8")), "{bit:?}");
    assert!(
        !bit.iter().any(|l| l.contains(".L.crctable")),
        "LLVM построил свою таблицу и при opt-level=s"
    );
    let table = mnemonics(&asm, TABLE);
    assert!(table.iter().any(|l| l.starts_with("ldrh")), "{table:?}");
    assert!(table.len() < bit.len(), "{table:?}\n{bit:?}");
}

/// Наблюдение, rustc 1.97 с LLVM 22: при `opt-level = 3` LLVM сам
/// узнаёт побитовый CRC и заменяет цикл своей таблицей `.L.crctable` —
/// тело `crc16_update` читает её так же, как `crc16_update_table` читает
/// `CRC16_TABLE`. Язык этого не обещает; другая версия LLVM может и не
/// узнать. На x86-64 в release (тоже `opt-level = 3`) — то же самое.
#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_observed_llvm_builds_crc_table_at_o3() {
    let asm = asm_of("ch25", "thumbv7m-none-eabi", "ch25-crc-3", &["-C", "opt-level=3"]);
    let bit = mnemonics(&asm, BITWISE);
    assert!(bit.iter().any(|l| l.contains(".L.crctable")), "{bit:?}");
    assert!(bit.iter().any(|l| l.starts_with("ldrh")), "{bit:?}");
    let asm = asm_of("ch25", "x86_64-unknown-linux-gnu", "ch25-crc-x86", &[]);
    let bit = mnemonics(&asm, BITWISE);
    assert!(bit.iter().any(|l| l.contains(".L.crctable(%rip)")), "{bit:?}");
}
