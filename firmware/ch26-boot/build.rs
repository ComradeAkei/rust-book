//! Свой скрипт компоновки boot.ld — в OUT_DIR под именем link.x, которое
//! ждёт ключ -Tlink.x из firmware/.cargo/config.toml.
use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("link.x"), include_bytes!("boot.ld")).unwrap();
    fs::write(out.join("memory.x"), include_bytes!("../memory.x")).unwrap();
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=boot.ld");
    println!("cargo::rerun-if-changed=../memory.x");
}
