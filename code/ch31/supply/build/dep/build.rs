// ANCHOR: build
//! Сценарий сборки зависимости — обычная программа. Cargo запускает её на
//! машине разработчика с его правами: при `cargo build`, `cargo check` и
//! когда редактор с rust-analyzer открывает проект. Эта только оставляет
//! след рядом с крейтом — вне OUT_DIR, куда ей писать положено, — и
//! запускает `id -u`, чтобы показать, от чьего имени работает.
use std::path::Path;
use std::process::Command;

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let uid = Command::new("id").arg("-u").output().unwrap().stdout;
    let mark = Path::new(&dir).join("..").join("build-rs-ran");
    std::fs::write(&mark, format!("сценарий сборки, uid {}", String::from_utf8_lossy(&uid).trim()))
        .unwrap();
    println!("cargo::rerun-if-changed=build.rs");
}
// ANCHOR_END: build
