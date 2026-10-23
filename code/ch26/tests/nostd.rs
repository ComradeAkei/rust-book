//! Разделы 26.1 и 26.6: что компилятор говорит о слоях core, alloc и
//! std. Фрагменты из `fragments/` собираются закреплённым rustc 1.97, и
//! тесты сверяют коды и тексты ошибок, приведённые в книге.
//!
//! Под Miri тесты пропускаются, в задании CI со случайной раскладкой —
//! по префиксу `rustc_`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// rustc закреплённой версии над фрагментом `file`; `args` — что
/// собрать. Возвращает успех и stderr.
fn rustc(file: &str, args: &[&str]) -> (bool, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch26-nostd").join(file);
    std::fs::create_dir_all(&out).unwrap();
    let res = Command::new("rustc")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .args(["--edition", "2024", "--color", "never", "--out-dir"])
        .arg(&out)
        .args(args)
        .arg(dir.join("fragments").join(file))
        .output()
        .unwrap();
    (res.status.success(), String::from_utf8_lossy(&res.stderr).into_owned())
}

fn lib(file: &str, target: &str) -> (bool, String) {
    rustc(file, &["--crate-type", "lib", "--emit", "metadata", "--target", target])
}

const HOST: &str = "x86_64-unknown-linux-gnu";
const M0: &str = "thumbv6m-none-eabi";

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_vec_is_not_in_core() {
    let (ok, err) = lib("vec.rs", HOST);
    assert!(!ok);
    assert!(err.contains("error[E0425]: cannot find type `Vec` in this scope"), "{err}");
    assert!(err.contains("error[E0433]: cannot find type `Vec` in this scope"), "{err}");
    let (ok, err) = lib("vec_alloc.rs", HOST);
    assert!(ok, "{err}");
    let (ok, err) = lib("vec_alloc.rs", M0);
    assert!(ok, "{err}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_no_std_on_cortex_m0() {
    let (ok, err) = lib("std_on_m0.rs", M0);
    assert!(!ok);
    assert!(err.contains("error[E0463]: can't find crate for `std`"), "{err}");
    assert!(
        err.contains("note: the `thumbv6m-none-eabi` target may not support the standard library"),
        "{err}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_alloc_needs_an_allocator() {
    let (ok, err) =
        rustc("no_allocator.rs", &["--crate-type", "bin", "--target", "thumbv7m-none-eabi", "-O"]);
    assert!(!ok);
    assert!(
        err.contains(
            "error: no global memory allocator found but one is required; link to std or add \
             `#[global_allocator]` to a static item that implements the GlobalAlloc trait"
        ),
        "{err}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_cortex_m0_has_no_rmw_atomics() {
    let (ok, err) = lib("atomics_m0.rs", M0);
    assert!(!ok);
    assert!(err.contains("error[E0432]: unresolved import `alloc::sync`"), "{err}");
    assert!(err.contains("could not find `sync` in `alloc`"), "{err}");
    assert!(
        err.contains("error[E0599]: no method named `fetch_add` found for reference `&Atomic<u32>` in the current scope"),
        "{err}"
    );
    // На Cortex-M3 то же самое собирается.
    let (ok, err) = lib("atomics_m0.rs", "thumbv7m-none-eabi");
    assert!(ok, "{err}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_panic_handler_in_lib_breaks_host_tests() {
    // Для Cortex-M библиотека собирается.
    let (ok, err) = lib("panic_in_lib.rs", "thumbv7m-none-eabi");
    assert!(ok, "{err}");
    // А тест на хосте — нет: обработчик уже есть в std.
    let (ok, err) = rustc("panic_in_lib.rs", &["--test"]);
    assert!(!ok);
    assert!(err.contains("error[E0152]: found duplicate lang item `panic_impl`"), "{err}");
    assert!(
        err.contains("the lang item is first defined in crate `std` (which `test` depends on)"),
        "{err}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_tests_link_std_but_do_not_import_it() {
    let (ok, err) = rustc("vec_in_test.rs", &["--test"]);
    assert!(!ok);
    assert!(err.contains("error: cannot find macro `vec` in this scope"), "{err}");
}
