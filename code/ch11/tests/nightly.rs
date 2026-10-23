//! Разделы 11.6 и 11.8: то, что есть только в nightly.
//!
//! - `ptr::metadata` и `DynMetadata`: размер и выравнивание из таблицы
//!   без чтения памяти наугад (задача D).
//! - `-Zretpoline`: косвенный вызов через переходник с `pause; lfence`,
//!   и запрет смешивать такой крейт с обычной `std`.
//!
//! Запуск: `cargo +nightly test -p ch11 --features nightly --test nightly`.
//! Без возможности `nightly` файл пуст и собирается на stable.
#![cfg(feature = "nightly")]
#![cfg_attr(feature = "nightly", feature(ptr_metadata))]

use ch11::any::{Fan, Node};
use ch11::fat::{Counter, Sensor, Thermo};
use std::alloc::Layout;
use std::process::Command;
use std::ptr::{self, DynMetadata};

#[test]
fn dyn_metadata() {
    let t = Thermo { raw: 16 };
    let s: &dyn Sensor = &t;
    let m: DynMetadata<dyn Sensor> = ptr::metadata(s);
    assert_eq!((m.size_of(), m.align_of()), (2, 2));
    assert_eq!(m.layout(), Layout::new::<Thermo>());
    // Разобрать и собрать обратно: тот же объект, та же таблица.
    let (data, meta) = (s as *const dyn Sensor).to_raw_parts();
    let back: *const dyn Sensor = ptr::from_raw_parts(data, meta);
    // SAFETY: back собран из частей живого указателя s.
    assert_eq!(unsafe { &*back }.read(), 100);
    let c: &dyn Sensor = &Counter { pulses: 1 };
    assert_eq!(ptr::metadata(c).layout(), Layout::new::<Counter>());
    // Таблица супертрейта внутри таблицы Node: те же размер и выравнивание.
    let n: &dyn Node = &Fan { rpm: 1, id: 2 };
    let up: &dyn Sensor = n;
    assert_eq!(ptr::metadata(up).layout(), ptr::metadata(n).layout());
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_retpoline() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch11-retpoline");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("poll.rs");
    std::fs::write(
        &src,
        "pub trait Sensor { fn read(&self) -> i32; }\n\
         #[unsafe(no_mangle)]\n\
         pub fn poll(s: &[&dyn Sensor]) -> i32 { s.iter().map(|s| s.read()).sum() }\n",
    )
    .unwrap();
    // rustc того же nightly, что запустил тест: rustup передаёт выбор
    // через RUSTUP_TOOLCHAIN.
    let rustc = |extra: &[&str]| {
        Command::new("rustc")
            .args(["--edition", "2024", "--crate-type", "lib", "-O", "--emit", "asm", "-o"])
            .arg(dir.join("poll.s"))
            .args(["-C", "llvm-args=-x86-asm-syntax=intel", "-Zretpoline"])
            .args(extra)
            .arg(&src)
            .output()
            .unwrap()
    };
    let out = rustc(&[]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(err.contains("mixing `-Zretpoline` will cause an ABI mismatch"), "{err}");
    let out = rustc(&["-Cunsafe-allow-abi-mismatch=retpoline"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let asm = std::fs::read_to_string(dir.join("poll.s")).unwrap();
    assert!(asm.contains("call\t__llvm_retpoline_r11"), "{asm}");
    assert!(!asm.contains("call\tqword ptr"), "косвенных вызовов быть не должно");
    let thunk = &asm[asm.find("__llvm_retpoline_r11:").unwrap()..];
    assert!(thunk.contains("pause") && thunk.contains("lfence"), "{thunk}");
}
