//! Сценарий сборки главы 18: компилирует `c/support.c` в статический архив и
//! просит компоновщик слинковать его с крейтом. Тот же приём «вызвать
//! системный C-компилятор из build.rs», что и в других главах, без крейта
//! `cc`. Нужен `cc`/`gcc` и `ar` на машине сборки.
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=c/support.c");
    println!("cargo::rerun-if-changed=c/support.h");
    println!("cargo::rerun-if-changed=build.rs");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let obj = out.join("support.o");
    let lib = out.join("libsupport.a");
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());

    let compiled = Command::new(&cc)
        .args(["-c", "-O2", "-fPIC", "-Wall", "-Wextra", "-std=c11"])
        .arg("c/support.c")
        .arg("-o")
        .arg(&obj)
        .status()
        .expect("не удалось запустить C-компилятор");
    assert!(compiled.success(), "компиляция c/support.c не удалась");

    let archived = Command::new("ar")
        .arg("rcs")
        .arg(&lib)
        .arg(&obj)
        .status()
        .expect("не удалось запустить ar");
    assert!(archived.success(), "ar не собрал архив libsupport.a");

    println!("cargo::rustc-link-search=native={}", out.display());
    println!("cargo::rustc-link-lib=static=support");
}
