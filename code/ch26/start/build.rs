// ANCHOR: build
//! Ключи компоновки: без crt1.o и crti.o из libc (-nostartfiles) и без
//! динамического загрузчика (-static). rustc сам добавляет
//! -nodefaultlibs, поэтому libc не подключается и так.
fn main() {
    println!("cargo::rustc-link-arg-bins=-nostartfiles");
    // Опыт раздела 26.2: с CH26_DYNAMIC=1 — без -static.
    if std::env::var_os("CH26_DYNAMIC").is_none() {
        println!("cargo::rustc-link-arg-bins=-static");
    }
    println!("cargo::rerun-if-env-changed=CH26_DYNAMIC");
}
// ANCHOR_END: build
