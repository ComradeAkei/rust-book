// Общие тесты задач главы 26.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch26/tests/ch26.rs` и `solutions/ch26/tests/ch26.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A собирает пробы закреплённым rustc под x86-64 и Cortex-M0, B —
// программу `tiny/` закреплённым cargo, D проверяет свои memcpy и memset
// на хосте (и под Miri) и ассемблер крейта под Cortex-M. Тесты, которые
// запускают rustc и cargo, начинаются с `rustc_` и под Miri пропускаются.
// Задача C — прошивка, её проверяет firmware/run-demos.sh.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Каталог крейта с задачами: заготовки или решения.
fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Каталог для файлов, которые тест создаёт сам.
fn tmp(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{}-{name}", env!("CARGO_PKG_NAME")));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Команда с окружением закреплённого тулчейна: rustup выберет версию по
/// rust-toolchain.toml, а не по тулчейну, которым запущен сам тест.
fn pinned(program: &str, dir: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env("CARGO_TERM_COLOR", "never");
    cmd
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use sut::a_homes::{ANSWERS, Home, OnM0};

    /// Проба на каждый элемент: `{p}` — путь крейта, `core`, `alloc` или
    /// `std`. Методы проверяются без пути: их видно, когда крейт подключён.
    const PROBES: [(&str, &str); 10] = [
        ("Duration", "pub fn f() -> {p}::time::Duration { {p}::time::Duration::from_millis(5) }"),
        ("Instant", "pub fn f() -> {p}::time::Instant { {p}::time::Instant::now() }"),
        ("<[T]>::sort", "pub fn f(v: &mut [u32]) { v.sort() }"),
        ("<[T]>::sort_unstable", "pub fn f(v: &mut [u32]) { v.sort_unstable() }"),
        ("f32::sqrt", "pub fn f(x: f32) -> f32 { x.sqrt() }"),
        ("f32::abs", "pub fn f(x: f32) -> f32 { x.abs() }"),
        ("HashMap", "pub fn f() -> {p}::collections::HashMap<u32, u32> { {p}::collections::HashMap::new() }"),
        ("BTreeMap", "pub fn f() -> {p}::collections::BTreeMap<u32, u32> { {p}::collections::BTreeMap::new() }"),
        ("Arc", "pub fn f() -> {p}::sync::Arc<u32> { {p}::sync::Arc::new(1) }"),
        (
            "AtomicU32::fetch_add",
            "pub fn f(a: &{p}::sync::atomic::AtomicU32) -> u32 { \
             a.fetch_add(1, {p}::sync::atomic::Ordering::Relaxed) }",
        ),
    ];

    /// Заголовок крейта-пробы для слоя.
    fn header(home: Home) -> &'static str {
        match home {
            Home::Core => "#![no_std]\n",
            Home::Alloc => "#![no_std]\nextern crate alloc;\n",
            _ => "",
        }
    }

    fn path(home: Home) -> &'static str {
        match home {
            Home::Core => "core",
            Home::Alloc => "alloc",
            _ => "std",
        }
    }

    /// Собирается ли проба `i` в слое `home` под `target`.
    fn compiles(i: usize, home: Home, target: &str) -> bool {
        let src = format!("{}{}\n", header(home), PROBES[i].1.replace("{p}", path(home)));
        let dir = tmp("a");
        let file = dir.join(format!("p{i}-{}-{target}.rs", path(home)));
        std::fs::write(&file, src).unwrap();
        let res = pinned("rustc", crate_dir())
            .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata", "--target", target, "-o"])
            .arg(file.with_extension("rmeta"))
            .arg(&file)
            .output()
            .unwrap();
        res.status.success()
    }

    /// Нижний слой, в котором проба собирается, и есть ли она на Cortex-M0.
    fn probe(i: usize) -> (Home, OnM0) {
        let host = "x86_64-unknown-linux-gnu";
        let home = [Home::Core, Home::Alloc, Home::Std]
            .into_iter()
            .find(|&h| compiles(i, h, host))
            .unwrap_or_else(|| panic!("проба {} не собирается даже со std", PROBES[i].0));
        let m0 = match home {
            Home::Std => OnM0::No,
            _ if compiles(i, home, "thumbv6m-none-eabi") => OnM0::Yes,
            _ => OnM0::No,
        };
        (home, m0)
    }

    #[test]
    fn task_a_answers_name_the_items_in_order() {
        let names: Vec<&str> = ANSWERS.iter().map(|a| a.0).collect();
        let probes: Vec<&str> = PROBES.iter().map(|p| p.0).collect();
        assert_eq!(names, probes);
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn rustc_a_answers_match() {
        let mut wrong = Vec::new();
        for (i, &(name, home, m0)) in ANSWERS.iter().enumerate() {
            let truth = probe(i);
            if (home, m0) != truth {
                wrong.push(format!("{name}: ответ {home:?}/{m0:?}, на деле {:?}/{:?}", truth.0, truth.1));
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;

    const ET_EXEC: u16 = 2;
    const PT_DYNAMIC: u32 = 2;
    const PT_INTERP: u32 = 3;

    /// Тип файла и типы сегментов из заголовков ELF64 little-endian.
    fn elf_kinds(b: &[u8]) -> (u16, Vec<u32>) {
        assert_eq!(&b[..4], b"\x7fELF", "не ELF");
        assert_eq!(b[4], 2, "не ELF64");
        let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let phoff = u64::from_le_bytes(b[0x20..0x28].try_into().unwrap()) as usize;
        let (phentsize, phnum) = (u16_at(0x36) as usize, u16_at(0x38) as usize);
        (u16_at(0x10), (0..phnum).map(|i| u32_at(phoff + i * phentsize)).collect())
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_b_tiny_says_hello_without_libc() {
        let target = tmp("tiny");
        let res = pinned("cargo", Path::new(sut::b_tiny::TINY))
            .args(["build", "--release", "--color", "never", "--target-dir"])
            .arg(&target)
            .output()
            .unwrap();
        assert!(res.status.success(), "{}", text(&res.stderr));
        let exe = target.join("release/tiny");
        let run = Command::new(&exe).output().unwrap();
        assert_eq!(text(&run.stdout), "Hello, world!\n");
        assert!(run.stderr.is_empty(), "{}", text(&run.stderr));
        assert!(run.status.success(), "{:?}", run.status);

        let bytes = std::fs::read(&exe).unwrap();
        let (kind, segments) = elf_kinds(&bytes);
        assert_eq!(kind, ET_EXEC, "нужен ET_EXEC: статический, не PIE");
        assert!(!segments.contains(&PT_INTERP), "есть PT_INTERP: программу запускает ld-linux");
        assert!(!segments.contains(&PT_DYNAMIC), "есть PT_DYNAMIC: программа динамическая");
        assert!(!bytes.windows(5).any(|w| w == b"GLIBC"), "в файле есть следы glibc");
        assert!(bytes.len() <= 1024, "{} байт: уменьшите до 1 КиБ", bytes.len());
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;
    use sut::d_mem::*;

    fn pattern(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + 3) as u8).collect()
    }

    /// Буфер, выровненный на 8: под ним лежат u64.
    fn aligned8(n: usize) -> Vec<u64> {
        vec![0xEEEE_EEEE_EEEE_EEEE; n.div_ceil(8)]
    }

    #[test]
    fn task_d_memcpy_every_alignment() {
        let src = pattern(64);
        for n in 0..=40 {
            for so in 0..4 {
                for d in 0..4 {
                    let mut dst = vec![0xEEu8; 64];
                    // SAFETY: обе области внутри своих буферов, не перекрываются.
                    let r = unsafe { memcpy(dst.as_mut_ptr().add(d), src.as_ptr().add(so), n) };
                    assert_eq!(r, dst.as_mut_ptr().wrapping_add(d), "memcpy возвращает dest");
                    assert_eq!(dst[d..d + n], src[so..so + n], "n = {n}, src + {so}, dest + {d}");
                    assert!(dst[..d].iter().chain(&dst[d + n..]).all(|&b| b == 0xEE), "запись мимо");
                }
            }
        }
    }

    #[test]
    fn task_d_aeabi_memcpy_variants() {
        let src = pattern(48);
        type Copy = unsafe extern "C" fn(*mut u8, *const u8, usize);
        let variants: [(&str, Copy); 3] =
            [("__aeabi_memcpy", __aeabi_memcpy), ("__aeabi_memcpy4", __aeabi_memcpy4), ("__aeabi_memcpy8", __aeabi_memcpy8)];
        for (name, f) in variants {
            for n in [0, 1, 3, 4, 7, 8, 13, 16, 31, 40] {
                let mut store = aligned8(48);
                let dst = store.as_mut_ptr().cast::<u8>();
                let mut s8 = aligned8(48);
                let sp = s8.as_mut_ptr().cast::<u8>();
                // SAFETY: оба буфера по 48 байт, выровнены на 8.
                unsafe {
                    sp.copy_from_nonoverlapping(src.as_ptr(), 48);
                    f(dst, sp, n);
                    assert_eq!(std::slice::from_raw_parts(dst, n), &src[..n], "{name}, n = {n}");
                    assert_eq!(*dst.add(n), 0xEE, "{name}: запись мимо");
                }
            }
        }
    }

    #[test]
    fn task_d_memmove_overlaps_both_ways() {
        type Move = unsafe extern "C" fn(*mut u8, *const u8, usize);
        let wrappers: [(&str, Move); 3] =
            [("__aeabi_memmove", __aeabi_memmove), ("__aeabi_memmove4", __aeabi_memmove4), ("__aeabi_memmove8", __aeabi_memmove8)];
        for n in 0..=33 {
            for from in 0..8 {
                for to in 0..8 {
                    let mut expected = pattern(48);
                    expected.copy_within(from..from + n, to);
                    let mut buf = pattern(48);
                    let p = buf.as_mut_ptr();
                    // SAFETY: обе области внутри buf, перекрытие допустимо.
                    let r = unsafe { memmove(p.add(to), p.add(from), n) };
                    assert_eq!(r, p.wrapping_add(to), "memmove возвращает dest");
                    assert_eq!(buf, expected, "memmove n = {n}, {from} → {to}");
                    if from % 8 == 0 && to % 8 == 0 {
                        for (name, f) in wrappers {
                            let mut store = aligned8(48);
                            let q = store.as_mut_ptr().cast::<u8>();
                            // SAFETY: 48 байт в буфере, адреса кратны 8.
                            unsafe {
                                q.copy_from_nonoverlapping(pattern(48).as_ptr(), 48);
                                f(q.add(to), q.add(from), n);
                                assert_eq!(std::slice::from_raw_parts(q, 48), &expected[..], "{name}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn task_d_memset_and_memclr() {
        for n in 0..=40 {
            for off in 0..4 {
                for (c, byte) in [(0, 0u8), (0xA5, 0xA5), (0x1FF, 0xFF), (-1, 0xFF)] {
                    let mut buf = vec![0x11u8; 48];
                    // SAFETY: область внутри buf.
                    let r = unsafe { memset(buf.as_mut_ptr().add(off), c, n) };
                    assert_eq!(r, buf.as_mut_ptr().wrapping_add(off), "memset возвращает dest");
                    assert!(buf[off..off + n].iter().all(|&b| b == byte), "memset c = {c:#x}, n = {n}");
                    assert!(buf[..off].iter().chain(&buf[off + n..]).all(|&b| b == 0x11), "запись мимо");
                }
            }
        }
    }

    #[test]
    fn task_d_aeabi_memset_argument_order() {
        type Set = unsafe extern "C" fn(*mut u8, usize, i32);
        type Clr = unsafe extern "C" fn(*mut u8, usize);
        let sets: [(&str, Set); 3] =
            [("__aeabi_memset", __aeabi_memset), ("__aeabi_memset4", __aeabi_memset4), ("__aeabi_memset8", __aeabi_memset8)];
        let clrs: [(&str, Clr); 3] =
            [("__aeabi_memclr", __aeabi_memclr), ("__aeabi_memclr4", __aeabi_memclr4), ("__aeabi_memclr8", __aeabi_memclr8)];
        for n in [0, 1, 4, 5, 8, 17, 32] {
            for (name, f) in sets {
                let mut store = aligned8(40);
                let p = store.as_mut_ptr().cast::<u8>();
                // SAFETY: 40 байт, выровнено на 8. Длина — второй аргумент.
                unsafe {
                    f(p, n, 0x5A);
                    let b = std::slice::from_raw_parts(p, 40);
                    assert!(b[..n].iter().all(|&x| x == 0x5A), "{name}(p, {n}, 0x5A)");
                    assert!(b[n..].iter().all(|&x| x == 0xEE), "{name}: длина и значение перепутаны?");
                }
            }
            for (name, f) in clrs {
                let mut store = aligned8(40);
                let p = store.as_mut_ptr().cast::<u8>();
                // SAFETY: 40 байт, выровнено на 8.
                unsafe {
                    f(p, n);
                    let b = std::slice::from_raw_parts(p, 40);
                    assert!(b[..n].iter().all(|&x| x == 0) && b[n..].iter().all(|&x| x == 0xEE), "{name}, n = {n}");
                }
            }
        }
    }

    /// Все функции задачи.
    const MEM: [&str; 15] = [
        "memcpy", "memmove", "memset",
        "__aeabi_memcpy", "__aeabi_memcpy4", "__aeabi_memcpy8",
        "__aeabi_memmove", "__aeabi_memmove4", "__aeabi_memmove8",
        "__aeabi_memset", "__aeabi_memset4", "__aeabi_memset8",
        "__aeabi_memclr", "__aeabi_memclr4", "__aeabi_memclr8",
    ];

    /// Ассемблер крейта задач под `target` в release.
    fn asm(target: &str) -> String {
        let dir = tmp(&format!("d-{target}"));
        let res = pinned("cargo", crate_dir())
            .args(["rustc", "--lib", "--release", "--color", "never", "--target", target, "--target-dir"])
            .arg(&dir)
            .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
            .output()
            .unwrap();
        assert!(res.status.success(), "{}", text(&res.stderr));
        let deps = dir.join(target).join("release/deps");
        let prefix = format!("{}-", env!("CARGO_PKG_NAME").replace('-', "_"));
        let file = std::fs::read_dir(&deps)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy();
                n.starts_with(&prefix) && n.ends_with(".s")
            })
            .max_by_key(|p| p.metadata().unwrap().modified().unwrap())
            .unwrap();
        std::fs::read_to_string(file).unwrap()
    }

    /// Куда функция `name` передаёт управление: `bl x`, `b x`, `b.w x`, кроме
    /// локальных меток. Одинаковые функции LLVM сливает, и вместо тела
    /// остаётся псевдоним `name = другая`.
    fn targets(asm: &str, name: &str) -> Vec<String> {
        let label = format!("\n{name}:\n");
        let Some(start) = asm.find(&label) else {
            let alias = format!("\n{name} = ");
            let at = asm.find(&alias).unwrap_or_else(|| panic!("в ассемблере нет {name}"));
            let other = asm[at + alias.len()..].lines().next().unwrap().trim();
            return targets(asm, other);
        };
        let rest = &asm[start + label.len()..];
        let end = rest.find(".Lfunc_end").unwrap();
        rest[..end]
            .lines()
            .filter_map(|l| {
                let mut w = l.split_whitespace();
                let op = w.next()?;
                let arg = w.next()?;
                (matches!(op, "bl" | "b" | "b.w") && !arg.starts_with(".L")).then(|| arg.to_string())
            })
            .collect()
    }

    fn no_recursion(target: &str) {
        let asm = asm(target);
        let mut wrong = Vec::new();
        for name in MEM {
            let allowed: &[&str] = if name.starts_with("__aeabi") { &["memcpy", "memmove", "memset"] } else { &[] };
            for t in targets(&asm, name) {
                if MEM.contains(&t.as_str()) && !allowed.contains(&t.as_str()) {
                    wrong.push(format!("{name} вызывает {t}"));
                }
            }
        }
        assert!(wrong.is_empty(), "{target}:\n{}", wrong.join("\n"));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_d_no_mem_calls_on_cortex_m3() {
        no_recursion("thumbv7m-none-eabi");
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_d_no_mem_calls_on_cortex_m0() {
        no_recursion("thumbv6m-none-eabi");
    }
}
