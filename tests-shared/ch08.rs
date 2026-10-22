// Общие тесты задач главы 8.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch08/tests/ch08.rs` и `solutions/ch08/tests/ch08.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A собирает пробы закреплённым rustc, задача B — сам крейт под
// Cortex-M0, задача C — программу на C через make, задача D — рабочее
// пространство d-station через cargo. Под Miri эти тесты пропускаются.
// Объектные файлы тесты читают сами, без nm и readelf: формат ELF и
// архивов ar разобран в модуле `elf` ниже.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Каталог крейта с задачами: заготовки или решения.
fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Каталог для файлов, которые тест создаёт сам.
fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{}-{name}", env!("CARGO_PKG_NAME")));
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
        .env("CARGO_TERM_COLOR", "never");
    cmd
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

/// Запустить программу и вернуть её стандартный вывод.
fn run(program: &Path) -> String {
    let out = Command::new(program).output().unwrap();
    assert!(out.status.success(), "{} упала:\n{}", program.display(), text(&out.stderr));
    text(&out.stdout)
}

// ---------------------------------------------------------------- ELF и ar

mod elf {
    /// Символ объектного файла.
    pub struct Sym {
        pub name: String,
        /// Номер секции; 0 — символ не определён здесь (SHN_UNDEF).
        pub shndx: usize,
        pub size: u64,
        pub global: bool,
    }

    /// Объектный файл ELF: имена секций и таблица символов. Только
    /// little-endian, 32 и 64 бита, то есть ARM и x86_64.
    pub struct Elf {
        pub sections: Vec<String>,
        pub symbols: Vec<Sym>,
    }

    fn uint(b: &[u8], at: usize, size: usize) -> u64 {
        b[at..at + size].iter().rev().fold(0, |v, &x| v << 8 | u64::from(x))
    }

    fn cstr(b: &[u8], at: usize) -> String {
        let end = b[at..].iter().position(|&c| c == 0).unwrap();
        String::from_utf8_lossy(&b[at..at + end]).into_owned()
    }

    impl Elf {
        pub fn parse(b: &[u8]) -> Elf {
            assert!(b.starts_with(b"\x7fELF"), "не ELF");
            assert_eq!(b[5], 1, "ожидался little-endian");
            let wide = b[4] == 2;
            // Смещения полей заголовков в ELF32 и ELF64 различаются.
            let w = if wide { 8 } else { 4 };
            let (shoff, shentsize, shnum, shstrndx) = if wide {
                (uint(b, 0x28, 8), uint(b, 0x3a, 2), uint(b, 0x3c, 2), uint(b, 0x3e, 2))
            } else {
                (uint(b, 0x20, 4), uint(b, 0x2e, 2), uint(b, 0x30, 2), uint(b, 0x32, 2))
            };
            let sh = |i: usize| (shoff + i as u64 * shentsize) as usize;
            // name, type, offset, size, link, entsize
            let field = |i: usize, f: usize| -> u64 {
                let base = sh(i);
                match f {
                    0 => uint(b, base, 4),
                    1 => uint(b, base + 4, 4),
                    2 => uint(b, base + 8 + 2 * w, w),
                    3 => uint(b, base + 8 + 3 * w, w),
                    4 => uint(b, base + 8 + 4 * w, 4),
                    _ => uint(b, base + 16 + 5 * w, w),
                }
            };
            let strtab = field(shstrndx as usize, 2) as usize;
            let sections: Vec<String> =
                (0..shnum as usize).map(|i| cstr(b, strtab + field(i, 0) as usize)).collect();
            let mut symbols = Vec::new();
            for i in 0..shnum as usize {
                if field(i, 1) != 2 {
                    continue; // не SHT_SYMTAB
                }
                let (off, size, ent) = (field(i, 2), field(i, 3), field(i, 5));
                let names = field(field(i, 4) as usize, 2) as usize;
                for s in (off..off + size).step_by(ent as usize).skip(1) {
                    let s = s as usize;
                    let (info, shndx, sz) = if wide {
                        (b[s + 4], uint(b, s + 6, 2), uint(b, s + 16, 8))
                    } else {
                        (b[s + 12], uint(b, s + 14, 2), uint(b, s + 8, 4))
                    };
                    symbols.push(Sym {
                        name: cstr(b, names + uint(b, s, 4) as usize),
                        shndx: shndx as usize,
                        size: sz,
                        global: info >> 4 != 0, // STB_GLOBAL или STB_WEAK
                    });
                }
            }
            Elf { sections, symbols }
        }

        pub fn section(&self, s: &Sym) -> &str {
            self.sections.get(s.shndx).map_or("?", String::as_str)
        }
    }

    /// Члены архива ar в формате GNU: имя и содержимое. Длинные имена
    /// лежат в члене `//`, таблица символов `/` пропускается.
    pub fn ar(b: &[u8]) -> Vec<(String, &[u8])> {
        assert!(b.starts_with(b"!<arch>\n"), "не архив ar");
        let mut pos = 8;
        let mut long: &[u8] = &[];
        let mut out = Vec::new();
        while pos + 60 <= b.len() {
            let h = &b[pos..pos + 60];
            let name = String::from_utf8_lossy(&h[..16]).trim_end().to_string();
            let size: usize = String::from_utf8_lossy(&h[48..58]).trim().parse().unwrap();
            let data = &b[pos + 60..pos + 60 + size];
            pos += 60 + size + size % 2;
            let name = match name.as_str() {
                "//" => {
                    long = data;
                    continue;
                }
                "/" | "/SYM64/" => continue,
                n => match n.strip_prefix('/').and_then(|o| o.parse::<usize>().ok()) {
                    Some(o) => {
                        let end = long[o..].iter().position(|&c| c == b'\n').unwrap();
                        String::from_utf8_lossy(&long[o..o + end]).trim_end_matches('/').to_string()
                    }
                    None => n.trim_end_matches('/').to_string(),
                },
            };
            out.push((name, data));
        }
        out
    }
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use sut::a_visibility::answers;

    /// Элементы по порядку из условия и обращение к каждому. `crate::`
    /// в другом крейте заменяется на `station::`.
    const PROBES: [(&str, &str); 8] = [
        ("CLOCK_HZ", "fn probe() -> u32 { crate::CLOCK_HZ }"),
        ("Uart::baud", "fn probe(u: &crate::hal::uart::Uart) -> u32 { u.baud }"),
        ("Uart::fifo", "fn probe(u: &crate::hal::uart::Uart) -> u8 { u.fifo[0] }"),
        ("Uart::reset", "fn probe(u: &mut crate::hal::uart::Uart) { u.reset() }"),
        (
            "hal::dma::start",
            "fn probe(c: &crate::hal::DmaChannel, u: &mut crate::hal::uart::Uart) { \
             crate::hal::dma::start(c, u) }",
        ),
        ("hal::DmaChannel", "fn probe(_: &crate::hal::DmaChannel) {}"),
        ("DmaChannel.0", "fn probe(c: &crate::hal::DmaChannel) -> u8 { c.0 }"),
        ("app::render", "fn probe() { crate::app::render() }"),
    ];

    const PLACES: [&str; 6] =
        ["корень", "hal", "hal::uart", "hal::dma", "app", "другой крейт"];

    fn fragment() -> String {
        std::fs::read_to_string(crate_dir().join("../../code/ch08/fragments/a.rs")).unwrap()
    }

    /// Вставить пробу в строку после метки ⟨n⟩.
    fn with_probe(src: &str, place: usize, probe: &str) -> String {
        let mark = format!("⟨{place}⟩");
        let mut out = String::new();
        let mut found = false;
        for line in src.lines() {
            out.push_str(line);
            out.push('\n');
            if line.trim_start().starts_with(&format!("// {mark}")) && !found {
                found = true;
                out.push_str(probe);
                out.push('\n');
            }
        }
        assert!(found, "нет метки {mark}");
        out
    }

    /// Собрать файл и вернуть успех и текст ошибок.
    fn rustc(dir: &Path, name: &str, src: &str, args: &[&str]) -> (bool, String) {
        let file = dir.join(format!("{name}.rs"));
        std::fs::write(&file, src).unwrap();
        let out = pinned("rustc", crate_dir())
            .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata"])
            .args(["-A", "warnings", "--crate-name", name, "-o"])
            .arg(dir.join(format!("lib{name}.rmeta")))
            .args(args)
            .arg(&file)
            .output()
            .expect("не удалось запустить rustc");
        (out.status.success(), text(&out.stderr))
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_a_visibility() {
        let answers = answers();
        for (row, line) in answers.iter().enumerate() {
            assert!(
                line.len() == 6 && line.bytes().all(|c| c == b'+' || c == b'-'),
                "элемент {} ({}): нужна строка из шести + и -, а не {line:?}",
                row + 1,
                PROBES[row].0
            );
        }
        let dir = tmp("a");
        let src = fragment();
        let (ok, err) = rustc(&dir, "station", &src, &[]);
        assert!(ok, "фрагмент не собирается:\n{err}");
        let station = format!("station={}", dir.join("libstation.rmeta").display());

        // Пробы по местам в отдельных потоках: 48 запусков rustc.
        let results: Vec<Vec<(bool, String)>> = std::thread::scope(|s| {
            let jobs: Vec<_> = (1..=6)
                .map(|place| {
                    let (dir, src, station) = (&dir, &src, &station);
                    s.spawn(move || {
                        PROBES
                            .iter()
                            .enumerate()
                            .map(|(i, (_, probe))| {
                                let name = format!("p{place}_{i}");
                                if place == 6 {
                                    let probe = format!("pub {}", probe.replace("crate::", "station::"));
                                    rustc(dir, &name, &probe, &["--extern", station])
                                } else {
                                    rustc(dir, &name, &with_probe(src, place, probe), &[])
                                }
                            })
                            .collect()
                    })
                })
                .collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });

        let mut wrong = Vec::new();
        for (row, (what, _)) in PROBES.iter().enumerate() {
            for (col, place) in PLACES.iter().enumerate() {
                let (ok, err) = &results[col][row];
                // Проба не должна ломаться ни на чём, кроме видимости.
                if !ok {
                    assert!(
                        ["E0603", "E0616", "E0624"].iter().any(|c| err.contains(c)),
                        "проба {what} из {place} сломана не видимостью:\n{err}"
                    );
                }
                let said = answers[row].as_bytes()[col] == b'+';
                if said != *ok {
                    wrong.push(format!(
                        "{what} из места ⟨{}⟩ ({place}): ответ «{}», а на деле {}",
                        col + 1,
                        if said { "видно" } else { "не видно" },
                        if *ok { "компилируется" } else { "ошибка" },
                    ));
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, TAU};
    use sut::b_sine::{SINE, sin_q15};

    #[test]
    fn task_b_table() {
        for (i, &v) in SINE.iter().enumerate() {
            let want = (32767.0 * (i as f64 / 256.0 * FRAC_PI_2).sin()).round() as i16;
            assert_eq!(v, want, "SINE[{i}]");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "65536 углов под Miri слишком долго")]
    fn task_b_accuracy() {
        assert_eq!(sin_q15(0), 0);
        assert_eq!(sin_q15(0x4000), 32767);
        assert_eq!(sin_q15(0x8000), 0);
        assert_eq!(sin_q15(0xc000), -32767);
        let mut worst = (0u16, 0.0f64);
        for a in 0..=u16::MAX {
            let want = 32767.0 * (f64::from(a) / 65536.0 * TAU).sin();
            let err = (f64::from(sin_q15(a)) - want).abs();
            if err > worst.1 {
                worst = (a, err);
            }
        }
        let (a, err) = worst;
        assert!(err <= 2.0, "угол {a:#06x}: ошибка {err:.2} единицы, а можно не больше 2");
    }

    /// Вызовы программной плавающей точки: ARM EABI и compiler-builtins.
    fn soft_float(name: &str) -> bool {
        let aeabi = name.strip_prefix("__aeabi_").is_some_and(|op| {
            op.starts_with(['f', 'd']) || op.starts_with("cf") || op.starts_with("cd")
                || op.ends_with("2f") || op.ends_with("2d")
        });
        aeabi
            || name.starts_with("__float")
            || name.starts_with("__fix")
            || name.ends_with("sf3")
            || name.ends_with("df3")
            || ["sin", "sinf", "cos", "cosf"].contains(&name)
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_b_flash() {
        let target = tmp("b");
        let out = pinned(env!("CARGO"), crate_dir())
            .args(["build", "--lib", "--release", "--target", "thumbv6m-none-eabi", "--target-dir"])
            .arg(&target)
            .output()
            .unwrap();
        assert!(out.status.success(), "сборка под Cortex-M0:\n{}", text(&out.stderr));
        let lib = format!("lib{}.rlib", env!("CARGO_PKG_NAME").replace('-', "_"));
        let bytes = std::fs::read(target.join("thumbv6m-none-eabi/release").join(lib)).unwrap();

        let mut table = Vec::new();
        let mut floats = Vec::new();
        for (_, data) in elf::ar(&bytes) {
            if !data.starts_with(b"\x7fELF") {
                continue; // lib.rmeta
            }
            let obj = elf::Elf::parse(data);
            for s in &obj.symbols {
                if s.name.contains("4SINE") && s.shndx != 0 {
                    table.push((obj.section(s).to_string(), s.size));
                }
                if s.shndx == 0 && soft_float(&s.name) {
                    floats.push(s.name.clone());
                }
            }
        }
        assert_eq!(table.len(), 1, "нет static SINE в объектном коде: {table:?}");
        let (section, size) = &table[0];
        assert!(
            section.starts_with(".rodata"),
            "SINE лежит в {section}: при запуске её скопируют или обнулят в ОЗУ"
        );
        assert_eq!(*size, 2 * 257);
        assert!(floats.is_empty(), "на Cortex-M0 вызывается плавающая точка: {floats:?}");
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use std::collections::BTreeSet;
    use sut::c_mangle::demangle;

    /// Настоящие имена из rustc 1.97: библиотека code/ch08 и std.
    const V0: [(&str, &str); 17] = [
        ("_RNvNtCs2zcVwUOuYdq_4ch087symbols4both", "ch08::symbols::both"),
        ("_RNvMNtCs2zcVwUOuYdq_4ch087symbolsNtB2_4Uart4send", "<ch08::symbols::Uart>::send"),
        (
            "_RINvNtCs2zcVwUOuYdq_4ch087symbols8checksumAhj4_EB4_",
            "ch08::symbols::checksum::<[u8; 4]>",
        ),
        (
            "_RINvNtCs2zcVwUOuYdq_4ch087symbols8checksumRShEB4_",
            "ch08::symbols::checksum::<&[u8]>",
        ),
        (
            "_RNCINvNtCs2zcVwUOuYdq_4ch087symbols8checksumRShE0B6_",
            "ch08::symbols::checksum::<&[u8]>::{closure#0}",
        ),
        (
            "_RINvXs2J_NtNtCscI6d9CVNmLh_4core5slice4iterINtB7_4IterhENtNtNtNtBb_4iter6traits8\
             iterator8Iterator4foldhNCINvNtCs2zcVwUOuYdq_4ch087symbols8checksumAhj4_E0EB1K_",
            "<core::slice::iter::Iter<u8> as core::iter::traits::iterator::Iterator>::fold::\
             <u8, ch08::symbols::checksum<[u8; 4]>::{closure#0}>",
        ),
        (
            "_RNvMs0_NtCscI6d9CVNmLh_4core3numl13from_le_bytesCs2zcVwUOuYdq_4ch08",
            "<i32>::from_le_bytes",
        ),
        (
            "_RNvMs14_NtNtCscI6d9CVNmLh_4core4sync6atomicINtB6_6AtomicmE9fetch_addCs2zcVwUOuYdq_4ch08",
            "<core::sync::atomic::Atomic<u32>>::fetch_add",
        ),
        (
            "_RNvNvMNtNtCscI6d9CVNmLh_4core3ptr9const_ptrPp20offset_from_unsigned18\
             precondition_checkCs2zcVwUOuYdq_4ch08",
            "<*const _>::offset_from_unsigned::precondition_check",
        ),
        (
            "_RNvXs1h_NtCscI6d9CVNmLh_4core3fmtQShNtB6_5Debug3fmtCs8RI9jBTgU6F_14rustc_demangle",
            "<&mut [u8] as core::fmt::Debug>::fmt",
        ),
        (
            "_RNvXsq_NtCscI6d9CVNmLh_4core3fmtONtNtB7_3ffi6c_voidNtB5_5Debug3fmtCsgczF5crJ4sT_3std",
            "<*mut core::ffi::c_void as core::fmt::Debug>::fmt",
        ),
        (
            "_RINvNtCscI6d9CVNmLh_4core9panicking13assert_failedAhj4_RShECsgczF5crJ4sT_3std",
            "core::panicking::assert_failed::<[u8; 4], &[u8]>",
        ),
        (
            "_RNvXNtCscI6d9CVNmLh_4core3anyuNtB2_3Any7type_idCsgczF5crJ4sT_3std",
            "<() as core::any::Any>::type_id",
        ),
        (
            "_RNCNCNCNvNtCsgczF5crJ4sT_3std2rt19lang_start_internal00s_0B9_",
            "std::rt::lang_start_internal::{closure#0}::{closure#0}::{closure#1}",
        ),
        (
            "_RNvNCNKNvNtNtCsgczF5crJ4sT_3std2io5stdio14OUTPUT_CAPTURE0023___RUST_STD_INTERNAL_VAL",
            "std::io::stdio::OUTPUT_CAPTURE::{K#0}::{closure#0}::__RUST_STD_INTERNAL_VAL",
        ),
        (
            "_RNSNvYNCNCNvNtNtCsgczF5crJ4sT_3std3sys9backtrace10__print_fmts_00INtNtNtCscI6d9CVNmLh_\
             4core3ops8function6FnOnceTRNtNtNtBe_12backtrace_rs9symbolize6SymbolEE9call_once6vtableBe_",
            "<std::sys::backtrace::_print_fmt::{closure#1}::{closure#0} as core::ops::function::\
             FnOnce<(&std::backtrace_rs::symbolize::Symbol,)>>::call_once::{shim:vtable#0}",
        ),
        (
            "_RNvNtNtCsgczF5crJ4sT_3std2io5stdio19OUTPUT_CAPTURE_USED.0",
            "std::io::stdio::OUTPUT_CAPTURE_USED (.0)",
        ),
    ];

    /// Те же функции code/ch08, собранные с -Csymbol-mangling-version=legacy.
    const LEGACY: [(&str, &str); 7] = [
        ("_ZN4ch087symbols4Uart4send17h144cbd767f8d92caE", "ch08::symbols::Uart::send"),
        ("_ZN4ch087symbols8checksum17h5e3d872b0ab18e3eE", "ch08::symbols::checksum"),
        (
            "_ZN4ch087symbols8checksum28_$u7b$$u7b$closure$u7d$$u7d$17h6f4494c0dd7bc8f6E",
            "ch08::symbols::checksum::{{closure}}",
        ),
        (
            "_ZN4core3num21_$LT$impl$u20$i32$GT$13from_le_bytes17hc0b8f6c0be852d1cE",
            "core::num::<impl i32>::from_le_bytes",
        ),
        (
            "_ZN4core3ptr9const_ptr33_$LT$impl$u20$$BP$const$u20$T$GT$20offset_from_unsigned18\
             precondition_check17h42154a13205c960aE",
            "core::ptr::const_ptr::<impl *const T>::offset_from_unsigned::precondition_check",
        ),
        (
            "_ZN55_$LT$$RF$T$u20$as$u20$core..convert..AsRef$LT$U$GT$$GT$6as_ref17hde017dc16af215c5E",
            "<&T as core::convert::AsRef<U>>::as_ref",
        ),
        (
            "_ZN4core5array92_$LT$impl$u20$core..convert..AsRef$LT$$u5b$T$u5d$$GT$$u20$for$u20$\
             $u5b$T$u3b$$u20$N$u5d$$GT$6as_ref17h8c24287c1a4857ecE",
            "core::array::<impl core::convert::AsRef<[T]> for [T; N]>::as_ref",
        ),
    ];

    /// Не имена Rust или испорченные имена.
    const BAD: [&str; 6] = [
        "station_crc8",
        "_Z3foov",
        "_ZN3foo3barEv",
        "_ZN4abcE",
        "_RNvC",
        "_RNvNtB0_4core4main",
    ];

    #[test]
    fn task_c_vectors() {
        let mut wrong = Vec::new();
        for (sym, want) in V0.iter().chain(&LEGACY) {
            let got = demangle(sym);
            if got.as_deref() != Some(*want) {
                wrong.push(format!("{sym}\n  ждали: {want}\n  вышло: {got:?}"));
            }
        }
        for sym in BAD {
            if let Some(got) = demangle(sym) {
                wrong.push(format!("{sym}: не имя Rust, а вышло {got:?}"));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает make, cargo и компилятор C")]
    fn task_c_from_c() {
        let out = tmp("c");
        let make = pinned("make", &crate_dir().join("c"))
            .arg(format!("OUT={}", out.display()))
            .output()
            .expect("не удалось запустить make");
        assert!(
            make.status.success(),
            "make:\n{}{}",
            text(&make.stdout),
            text(&make.stderr)
        );
        assert_eq!(
            run(&out.join("station-demo")),
            "crc8(\"123456789\") = 0xf4\n\
             crc8(пусто) = 0x00\n\
             датчик 1: 1000 -> 10000\n\
             датчик 7: 1000 -> 25000\n\
             датчик 12: 1000 -> 2000\n\
             датчик 3: нет в таблице\n"
        );

        // В архиве: объектные файлы самого крейта и всё, от чего он зависит.
        let bytes = std::fs::read(out.join("target/release/libstation.a")).unwrap();
        let members = elf::ar(&bytes);
        for krate in ["std-", "core-", "compiler_builtins-"] {
            assert!(members.iter().any(|(n, _)| n.starts_with(krate)), "в архиве нет {krate}*");
        }
        let own: Vec<elf::Elf> = members
            .iter()
            .filter(|(n, _)| n.starts_with("station-"))
            .map(|(_, d)| elf::Elf::parse(d))
            .collect();
        let syms = || own.iter().flat_map(|o| &o.symbols);
        for f in ["station_crc8", "station_scale"] {
            assert!(
                syms().any(|s| s.name == f && s.global && s.shndx != 0),
                "{f} не экспортирована под именем из station.h"
            );
        }

        // Все имена Rust из объектных файлов крейта разбираются.
        let mut names = BTreeSet::new();
        for s in syms().filter(|s| s.name.starts_with("_R")) {
            match demangle(&s.name) {
                Some(d) => names.insert(d),
                None => panic!("не разобрано имя из libstation.a: {}", s.name),
            };
        }
        for want in ["<station::Scale>::apply", "station::scale_for", "station::SCALES"] {
            assert!(names.contains(want), "нет {want} среди {names:#?}");
        }
        assert!(names.iter().any(|n| n.starts_with("station::crc8::<")), "{names:#?}");
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;

    const PANEL: &str = "табло: 21.50 °C\nтабло: -4.05 °C\nтабло: 0.00 °C\n";
    const EXPORT: &str = "выгрузка: 70.70 °F\nвыгрузка: 24.71 °F\nвыгрузка: 32.00 °F\n";

    fn cargo(target: &Path, args: &[&str]) -> Output {
        pinned(env!("CARGO"), &crate_dir().join("d-station"))
            .args(args)
            .arg("--target-dir")
            .arg(target)
            .output()
            .unwrap()
    }

    fn check(what: &str, out: &Output) {
        assert!(out.status.success(), "{what}:\n{}", text(&out.stderr));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_d_alone() {
        for (pkg, want) in [("panel", PANEL), ("export", EXPORT)] {
            let target = tmp(&format!("d-{pkg}"));
            check(&format!("cargo build -p {pkg}"), &cargo(&target, &["build", "-q", "-p", pkg]));
            assert_eq!(run(&target.join("debug").join(pkg)), want, "{pkg}, собранный один");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_d_workspace() {
        let target = tmp("d-all");
        check("cargo build --workspace", &cargo(&target, &["build", "-q", "--workspace"]));
        assert_eq!(run(&target.join("debug/panel")), PANEL, "panel в сборке всего пространства");
        assert_eq!(run(&target.join("debug/export")), EXPORT, "export в сборке всего пространства");
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_d_features() {
        let target = tmp("d-units");
        for set in [&["--no-default-features"][..], &["--all-features"]] {
            let mut args = vec!["check", "-q", "-p", "units"];
            args.extend_from_slice(set);
            check(&format!("units с {}", set[0]), &cargo(&target, &args));
        }
    }
}
