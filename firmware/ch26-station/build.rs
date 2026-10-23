//! Скрипт компоновки — тот же ch26.ld, что у среды ch26-bare: таблица
//! векторов по адресу 0 и запрет изменяемых static.
use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("link.x"), include_bytes!("../ch26-bare/ch26.ld")).unwrap();
    fs::write(out.join("memory.x"), include_bytes!("../memory.x")).unwrap();
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=../ch26-bare/ch26.ld");
    println!("cargo::rerun-if-changed=../memory.x");
}
