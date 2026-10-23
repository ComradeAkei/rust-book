//! Разделы 27.3 и 27.4: ошибки работы с периферией, которые ловит
//! проверка типов.
//! Пробы — маленькие крейты, собранные закреплённым rustc против
//! библиотеки ch27-hal: верная — собирается, остальные — нет, с нужным
//! кодом ошибки. Под Miri тесты пропускаются, в задании CI со случайной
//! раскладкой — по префиксу `rustc_`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn tmp() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch27-hal-typestate");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Команда закреплённого тулчейна: rustup выберет версию по
/// rust-toolchain.toml, а не по тулчейну, которым запущен сам тест.
fn pinned(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env("CARGO_TERM_COLOR", "never");
    cmd
}

/// Библиотека HAL для хоста; собирается один раз на все пробы.
fn rlib() -> &'static Path {
    static RLIB: OnceLock<PathBuf> = OnceLock::new();
    RLIB.get_or_init(|| {
        let target = tmp().join("target");
        let out = pinned("cargo")
            .args(["build", "-p", "ch27-hal", "--lib", "--target-dir"])
            .arg(&target)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        target.join("debug")
    })
}

/// Тело функции `probe(dp)` с общим началом: такты, порты A и F.
fn probe(name: &str, body: &str) -> (bool, String) {
    let src = format!(
        "use ch27_hal::pac;\n\
         use ch27_hal::prelude::*;\n\
         use ch27_hal::serial::{{self, Serial}};\n\
         use ch27_hal::sysctl::Sysctl;\n\
         pub fn probe(dp: pac::Peripherals) {{\n\
         let mut sc = Sysctl::new(dp.sysctl);\n\
         let pa = dp.gpioa.split(&mut sc);\n\
         let pf = dp.gpiof.split(&mut sc);\n\
         {body}\n}}\n"
    );
    let dir = tmp();
    let file = dir.join(format!("{name}.rs"));
    std::fs::write(&file, src).unwrap();
    let lib = rlib();
    let out = pinned("rustc")
        .args(["--edition", "2024", "--crate-type", "lib", "-A", "warnings"])
        .args(["--crate-name", name, "--out-dir"])
        .arg(&dir)
        .arg("--extern")
        .arg(format!("ch27_hal={}", lib.join("libch27_hal.rlib").display()))
        .arg("-L")
        .arg(format!("dependency={}", lib.join("deps").display()))
        .arg(&file)
        .output()
        .unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo и rustc")]
fn rustc_correct_use_builds() {
    let (ok, err) = probe(
        "ts_ok",
        "let mut led = pf.pf0.into_output();\n\
         let Ok(()) = led.set_high();\n\
         let pins = (pa.pa0.into_alternate(), pa.pa1.into_alternate());\n\
         let _uart = Serial::new(dp.uart0, pins, serial::baud(115_200), &mut sc);",
    );
    assert!(ok, "верная проба не собралась:\n{err}");
}

// ANCHOR: errors
#[test]
#[cfg_attr(miri, ignore = "запускает cargo и rustc")]
fn rustc_mistakes_are_type_errors() {
    let cases = [
        // Вывод после сброса — вход, а у входа нет set_high.
        ("ts_input", "let mut led = pf.pf0;\nled.set_high();", "error[E0599]"),
        // Вывод уже отдан: второй смены режима быть не может.
        (
            "ts_moved",
            "let _led = pf.pf0.into_output();\nlet _again = pf.pf0.into_input();",
            "error[E0382]",
        ),
        // Порт уже разделён: значение dp.gpiof поглощено.
        ("ts_split", "let _pf2 = dp.gpiof.split(&mut sc);", "error[E0382]"),
        // Приём и передача UART перепутаны.
        (
            "ts_swapped",
            "let pins = (pa.pa1.into_alternate(), pa.pa0.into_alternate());\n\
             let _uart = Serial::new(dp.uart0, pins, serial::baud(115_200), &mut sc);",
            "error[E0308]",
        ),
        // Выводы не в режиме периферии.
        (
            "ts_mode",
            "let pins = (pa.pa0.into_output(), pa.pa1.into_output());\n\
             let _uart = Serial::new(dp.uart0, pins, serial::baud(115_200), &mut sc);",
            "error[E0308]",
        ),
        // Вывод нельзя создать из ничего: поле маркера закрыто.
        (
            "ts_forge",
            "let _fake: ch27_hal::gpio::PF1<ch27_hal::gpio::Output> =\n\
             ch27_hal::gpio::Pin { _mode: core::marker::PhantomData };",
            "error[E0451]",
        ),
    ];
    for (name, body, code) in cases {
        let (ok, err) = probe(name, body);
        assert!(!ok, "{name}: собралось, а не должно");
        assert!(err.contains(code), "{name}: ждали {code}:\n{err}");
    }
}
// ANCHOR_END: errors

// ANCHOR: pac
/// Раздел 27.3: что PAC разрешает без `unsafe`. Поле, у которого в SVD
/// есть ограничение записи, пишется `set`; поле без него — только
/// `unsafe bits`; регистр целиком — тоже `unsafe bits`; поля только для
/// чтения у записи нет вовсе.
#[test]
#[cfg_attr(miri, ignore = "запускает cargo и rustc")]
fn rustc_pac_raw_bits_need_unsafe() {
    let (ok, err) = probe(
        "pac_ok",
        "let u = &dp.uart0;\n\
         u.fbrd().write(|w| w.divfrac().set(33));\n\
         // SAFETY: 2 — порог «половина FIFO», допустимое значение поля.\n\
         u.ifls().write(|w| unsafe { w.txiflsel().bits(2) });\n\
         let _fe = u.dr().read().fe().bit_is_set();",
    );
    assert!(ok, "верная проба не собралась:\n{err}");
    let cases = [
        // Поле без ограничения записи: bits небезопасна.
        ("pac_field_bits", "dp.uart0.ifls().write(|w| w.txiflsel().bits(2));", "error[E0133]"),
        // А безопасного set у такого поля нет.
        ("pac_field_set", "dp.uart0.ifls().write(|w| w.txiflsel().set(2));", "error[E0599]"),
        // Слово целиком — всегда unsafe.
        ("pac_reg_bits", "dp.uart0.ctl().write(|w| w.bits(0x301));", "error[E0133]"),
        // FE только для чтения: в записи такого поля нет.
        ("pac_read_only", "dp.uart0.dr().write(|w| w.fe().set_bit());", "error[E0599]"),
    ];
    for (name, body, code) in cases {
        let (ok, err) = probe(name, body);
        assert!(!ok, "{name}: собралось, а не должно");
        assert!(err.contains(code), "{name}: ждали {code}:\n{err}");
    }
}
// ANCHOR_END: pac
