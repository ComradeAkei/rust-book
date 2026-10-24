//! Кладёт свой скрипт компоновки в OUT_DIR под именем link.x: ключ
//! -Tlink.x из firmware/.cargo/config.toml найдёт его, а не скрипт
//! cortex-m-rt, от которого эта прошивка не зависит.
//!
//! Опыт раздела 26.4: с CH26_NO_GUARD=1 из скрипта вырезаются блоки
//! GUARD — секция-ловушка для .data и .bss и проверка её размера.
use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut script = String::from_utf8(include_bytes!("ch26.ld").to_vec()).unwrap();
    if env::var_os("CH26_NO_GUARD").is_some() {
        let mut keep = true;
        script = script
            .lines()
            .filter(|l| {
                if l.contains("/* GUARD") {
                    keep = false;
                }
                let k = keep;
                if l.contains("END GUARD */") {
                    keep = true;
                }
                k
            })
            .map(|l| format!("{l}\n"))
            .collect();
    }
    fs::write(out.join("link.x"), script).unwrap();
    fs::write(out.join("memory.x"), include_bytes!("../memory.x")).unwrap();
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=ch26.ld");
    println!("cargo::rerun-if-changed=../memory.x");
    println!("cargo::rerun-if-env-changed=CH26_NO_GUARD");
}
