// ANCHOR: build
//! Ключи компоновки. Без crt*.o из libc, без загрузчика, без Build ID и со
//! своим скриптом: один сегмент, заголовки внутри него, ничего лишнего.
fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let script = format!("-Wl,-T,{dir}/tiny.ld");
    for arg in ["-nostartfiles", "-static", "-Wl,--build-id=none", &script] {
        println!("cargo::rustc-link-arg-bins={arg}");
    }
    println!("cargo::rerun-if-changed=tiny.ld");
}
// ANCHOR_END: build
