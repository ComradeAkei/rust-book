//! Раздел 31.5: C-функция для опыта с ASan по ту сторону FFI. Когда крейт
//! собирают с `-Zsanitizer=address`, C тоже должен быть инструментирован:
//! иначе ASan не видит обращений из него. Флаг добавлять не нужно: cc 1.10
//! видит `address` в `CARGO_CFG_SANITIZE` (cargo задаёт её на nightly) и
//! сам передаёт компилятору C `-fsanitize=address` — на x86-64 Linux и
//! для GCC, и для clang. `CFLAGS=-fno-sanitize=address` его отменяет.

// ANCHOR: build_rs
fn main() {
    println!("cargo::rerun-if-changed=c/fill.c");
    cc::Build::new().file("c/fill.c").compile("ch31fill");
}
// ANCHOR_END: build_rs
