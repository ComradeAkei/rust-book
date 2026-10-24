//! Кладёт общую карту памяти рядом с link.x из cortex-m-rt и передаёт
//! прошивке уровень оптимизации: она печатает его рядом с замером.
use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("memory.x"), include_bytes!("../memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=../memory.x");
    println!("cargo:rustc-env=CH30_OPT_LEVEL={}", env::var("OPT_LEVEL").unwrap());
}
