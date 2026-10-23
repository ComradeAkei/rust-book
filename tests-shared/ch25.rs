// Общие тесты задач главы 25.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch25/tests/ch25.rs` и `solutions/ch25/tests/ch25.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A собирает фрагмент code/ch25/fragments/a_sections.rs закреплённым
// rustc под thumbv7m-none-eabi, B — сам крейт под ту же цель, C — пробы
// против крейта, D — крейт с урезанным бюджетом ОЗУ. Эти тесты запускают
// rustc и cargo и под Miri пропускаются; остальное — обычный код.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

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
        .env_remove("CH25_RAM_BUDGET")
        .env("CARGO_TERM_COLOR", "never");
    cmd
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

/// Имя библиотеки крейта в именах файлов: `ch25-solutions` → `ch25_solutions`.
fn lib_name() -> String {
    env!("CARGO_PKG_NAME").replace('-', "_")
}

/// Собрать библиотеку крейта задач закреплённым cargo в отдельный каталог.
fn cargo_build(name: &str, args: &[&str], env: &[(&str, &str)]) -> (PathBuf, Output) {
    let target = tmp(name);
    let out = pinned("cargo", crate_dir())
        .envs(env.iter().copied())
        .args(["build", "--lib", "--target-dir"])
        .arg(&target)
        .args(args)
        .output()
        .unwrap();
    (target, out)
}

/// Разбор ELF и архивов ar: символы и имена секций. Адреса символов и
/// разбор .rlib целиком нужны тестам главы 10, здесь — нет.
#[allow(dead_code)]
mod elf {
    include!("elf.rs");
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use sut::a_sections::{ANSWERS, Section};

    fn section(name: &str) -> Section {
        if name.starts_with(".rodata") {
            Section::Rodata
        } else if name.starts_with(".data") {
            Section::Data
        } else if name.starts_with(".bss") {
            Section::Bss
        } else {
            Section::Unknown
        }
    }

    #[test]
    fn a_names() {
        let names = ANSWERS.map(|(name, _)| name);
        assert_eq!(names, ["GAINS", "ZEROS", "BOOTS", "EVENTS", "SCRATCH", "CALIB"]);
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn rustc_a_sections_match() {
        let src = crate_dir().join("../../code/ch25/fragments/a_sections.rs");
        let obj = tmp("a").join("a_sections.o");
        let out = pinned("rustc", crate_dir())
            .args(["--edition", "2024", "--crate-type", "lib", "--crate-name", "a_sections"])
            .args(["--target", "thumbv7m-none-eabi", "--emit", "obj", "-C", "opt-level=s", "-o"])
            .arg(&obj)
            .arg(&src)
            .output()
            .unwrap();
        assert!(out.status.success(), "фрагмент не собирается:\n{}", text(&out.stderr));
        let elf = elf::parse(&std::fs::read(&obj).unwrap());
        let wrong: Vec<String> = ANSWERS
            .iter()
            .filter_map(|&(name, answer)| {
                let sym = elf
                    .symbols
                    .iter()
                    .find(|s| s.name == name && s.shndx != 0)
                    .unwrap_or_else(|| panic!("в объектном файле нет {name}"));
                let sec = &elf.sections[sym.shndx];
                (section(sec) != answer)
                    .then(|| format!("{name}: ответ {answer:?}, а rustc положил в {sec}"))
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;
    use sut::b_crc::{CHECK, CRC32_TABLE, POLY, SINE, crc32, crc32_table, sine_table};

    /// Побитовый CRC-32 без таблицы — эталон.
    fn bitwise(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }

    #[test]
    fn b_crc32_table() {
        assert_eq!(POLY, 0xEDB8_8320);
        assert_eq!([CRC32_TABLE[0], CRC32_TABLE[1], CRC32_TABLE[255]], [0, 0x7707_3096, 0x2D02_EF8D]);
        assert_eq!(crc32_table(POLY), CRC32_TABLE);
        assert_eq!((CHECK, crc32(b"123456789")), (0xCBF4_3926, 0xCBF4_3926));
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn b_crc32_matches_bitwise() {
        // Все однобайтовые и двухбайтовые входы и длинные данные.
        for a in 0..=255u8 {
            assert_eq!(crc32(&[a]), bitwise(&[a]), "[{a}]");
            for b in [0, 1, 0x7F, 0x80, 0xFF, a] {
                assert_eq!(crc32(&[a, b]), bitwise(&[a, b]), "[{a}, {b}]");
            }
        }
        let long: Vec<u8> = (0..3000u32).map(|i| i.wrapping_mul(2_654_435_761) as u8).collect();
        assert_eq!(crc32(&long), bitwise(&long));
    }

    /// Таблица из `const fn` совпадает с той, что в задаче 8.B считал
    /// `build.rs` через `f64::sin` и `round`.
    #[test]
    fn b_sine_matches_build_rs() {
        assert_eq!(SINE, ch08_solutions::b_sine::SINE);
        assert_eq!(sine_table(), SINE);
        assert_eq!([SINE[0], SINE[128], SINE[256]], [0, 23170, 32767]);
    }

    /// Обе таблицы — неизменяемые данные во flash: секции `.rodata.*`
    /// в объектных файлах крейта, собранного под Cortex-M3.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_b_tables_in_rodata() {
        let (target, out) =
            cargo_build("b", &["--release", "--target", "thumbv7m-none-eabi"], &[]);
        assert!(out.status.success(), "{}", text(&out.stderr));
        let rlib = target.join(format!("thumbv7m-none-eabi/release/lib{}.rlib", lib_name()));
        let bytes = std::fs::read(&rlib).unwrap();
        let mut found = Vec::new();
        for member in elf::ar(&bytes).into_iter().filter(|m| m.starts_with(b"\x7fELF")) {
            let elf = elf::parse(member);
            for s in elf.symbols.iter().filter(|s| s.shndx != 0) {
                for (what, size) in [("CRC32_TABLE", 1024), ("SINE", 514)] {
                    // Схема v0: путь из длин и имён, `…5b_crc11CRC32_TABLE`.
                    if s.name.ends_with(&format!("5b_crc{}{what}", what.len())) {
                        found.push((what, elf.sections[s.shndx].clone(), s.size));
                        assert_eq!(s.size, size, "{what}");
                    }
                }
            }
        }
        found.sort();
        assert_eq!(found.len(), 2, "{found:?}");
        for (what, sec, _) in &found {
            assert!(sec.starts_with(".rodata"), "{what} в {sec}, а не в .rodata");
        }
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use core::net::Ipv4Addr;
    use sut::c_ip::{IpError, ipv4, parse_ipv4};

    #[test]
    fn c_parse() {
        let cases: &[(&str, Result<[u8; 4], IpError>)] = &[
            ("192.168.0.1", Ok([192, 168, 0, 1])),
            ("0.0.0.0", Ok([0; 4])),
            ("255.255.255.255", Ok([255; 4])),
            ("10.0.0.300", Err(IpError::Range)),
            ("10.0.0.2560000000000", Err(IpError::Range)),
            ("1.2.3", Err(IpError::Count)),
            ("1.2.3.4.5", Err(IpError::Count)),
            ("1..2.3", Err(IpError::Empty)),
            ("", Err(IpError::Empty)),
            (".1.2.3", Err(IpError::Empty)),
            ("1.2.3.4.", Err(IpError::Empty)),
            ("1.2.3.a", Err(IpError::Digit)),
            (" 1.2.3.4", Err(IpError::Digit)),
            ("1.2.3.+4", Err(IpError::Digit)),
            ("01.2.3.4", Err(IpError::LeadingZero)),
            ("1.2.3.00", Err(IpError::LeadingZero)),
        ];
        for &(s, want) in cases {
            assert_eq!(parse_ipv4(s), want, "{s:?}");
            // Разбор стандартной библиотеки согласен, адрес это или нет.
            assert_eq!(s.parse::<Ipv4Addr>().ok().map(|a| a.octets()), want.ok(), "{s:?}");
        }
    }

    #[test]
    fn c_runtime() {
        assert_eq!(ipv4("10.1.2.3"), Ipv4Addr::new(10, 1, 2, 3));
    }

    #[test]
    #[should_panic(expected = "IPv4: октет больше 255")]
    fn c_runtime_panics() {
        let _ = ipv4("300.1.1.1");
    }

    /// Пробы против самого крейта: отладочная сборка в отдельном каталоге,
    /// затем rustc с `--extern sut=…`. Вернуть, собралась ли проба, и
    /// текст ошибок. Проба собирается полностью, `--emit link`: блок
    /// `const` в теле функции rustc 1.97 вычисляет при сборке, а при
    /// `--emit metadata` (так работает `cargo check`) — нет.
    fn probe(name: &str, src: &str) -> (bool, String) {
        static RLIB: OnceLock<PathBuf> = OnceLock::new();
        let rlib = RLIB.get_or_init(|| {
            let (target, out) = cargo_build("probes", &[], &[]);
            assert!(out.status.success(), "{}", text(&out.stderr));
            target.join(format!("debug/lib{}.rlib", lib_name()))
        });
        let dir = tmp("probes");
        let file = dir.join(format!("{name}.rs"));
        std::fs::write(&file, src).unwrap();
        let out = pinned("rustc", crate_dir())
            .args(["--edition", "2024", "--crate-type", "lib", "--emit", "link"])
            .args(["-A", "warnings", "--crate-name", name, "--out-dir"])
            .arg(&dir)
            .arg("--extern")
            .arg(format!("sut={}", rlib.display()))
            .arg(&file)
            .output()
            .unwrap();
        (out.status.success(), text(&out.stderr))
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo и rustc")]
    fn rustc_c_macro() {
        let ok = "use core::net::Ipv4Addr;\n\
                  pub const GATEWAY: Ipv4Addr = sut::ip!(\"192.168.0.1\");\n\
                  pub fn server() -> Ipv4Addr { sut::ip!(\"10.0.0.7\") }\n";
        let (built, err) = probe("c_ok", ok);
        assert!(built, "верные адреса не собрались:\n{err}");
        // Ошибка в адресе внутри обычной функции — ошибка сборки.
        let bad = [
            ("c_range", "10.0.0.300", "IPv4: октет больше 255"),
            ("c_count", "10.0.7", "IPv4: нужно ровно четыре октета"),
            ("c_zero", "10.0.0.07", "IPv4: ведущий ноль в октете"),
        ];
        let results: Vec<(bool, String)> = std::thread::scope(|s| {
            let jobs: Vec<_> = bad
                .iter()
                .map(|&(name, addr, _)| {
                    let src = format!(
                        "pub fn server() -> core::net::Ipv4Addr {{ sut::ip!({addr:?}) }}\n"
                    );
                    s.spawn(move || probe(name, &src))
                })
                .collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });
        for ((name, addr, msg), (built, err)) in bad.iter().zip(&results) {
            assert!(!built, "{name}: ip!({addr:?}) собрался");
            assert!(err.contains("error[E0080]") && err.contains(msg), "{name}:\n{err}");
        }
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;
    use sut::d_budget::{BUDGET, DRIVERS, USED, parse_budget, total};

    #[test]
    fn d_budget_from_memory_x() {
        assert_eq!((parse_budget("12288"), parse_budget("0"), parse_budget("007")), (12288, 0, 7));
        // RAM 16K из firmware/memory.x минус 4 КиБ под стек.
        assert_eq!(BUDGET, 16 * 1024 - 4096);
    }

    #[test]
    #[should_panic(expected = "CH25_RAM_BUDGET: не десятичное число")]
    fn d_budget_rejects_suffix() {
        let _ = parse_budget("12K");
    }

    #[test]
    fn d_drivers() {
        let names = DRIVERS.each_ref().map(|d| d.name);
        let ram = DRIVERS.each_ref().map(|d| d.ram);
        assert_eq!(names, ["uart", "adc", "log", "radio"]);
        assert_eq!(ram, [768, 2048, 4096, 1024]);
        assert_eq!((total(&DRIVERS), USED), (7936, 7936));
        assert_eq!(total(&[]), 0);
    }

    /// Бюджет меньше суммы буферов — сборка не проходит, и rustc называет
    /// причину и число лишних байт. Ровно по сумме — проходит.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_d_over_budget_stops_build() {
        let (_, out) = cargo_build("d", &[], &[("CH25_RAM_BUDGET", "4000")]);
        let err = text(&out.stderr);
        assert!(!out.status.success(), "бюджет 4000 байт, а крейт собрался");
        assert!(err.contains("буферы драйверов не помещаются в бюджет ОЗУ"), "{err}");
        assert!(err.contains("found one with a size of 3936"), "{err}");
        let (_, out) = cargo_build("d", &[], &[("CH25_RAM_BUDGET", "7936")]);
        assert!(out.status.success(), "бюджет ровно по буферам:\n{}", text(&out.stderr));
    }
}
