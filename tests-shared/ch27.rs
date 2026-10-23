// Общие тесты задач главы 27.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch27/tests/ch27.rs` и `solutions/ch27/tests/ch27.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A собирает прошивку firmware/ch27-nvic закреплённым cargo и
// разбирает её ELF, B — сам крейт задач под thumbv7m-none-eabi с
// ассемблером. Эти тесты под Miri пропускаются, в задании CI со
// случайной раскладкой — по префиксу `rustc_`. C, D1 и D2 — обычный код.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

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
        .env("CARGO_TERM_COLOR", "never");
    cmd
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Разбор ELF: символы с адресами.
#[allow(dead_code)]
mod elf {
    include!("elf.rs");
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use sut::a_vectors::{VtError, vector_table};

    /// ELF прошивки ch27-nvic: обработчики GPIOA, GPIOB, GPIOC, SVCall и
    /// HardFault свои, остальные — DefaultHandler.
    fn firmware() -> &'static [u8] {
        static ELF: OnceLock<Vec<u8>> = OnceLock::new();
        ELF.get_or_init(|| {
            let target = tmp("firmware");
            let out = pinned("cargo", &root().join("firmware"))
                .args(["build", "--release", "-p", "ch27-nvic", "--target-dir"])
                .arg(&target)
                .output()
                .unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
            std::fs::read(target.join("thumbv7m-none-eabi/release/ch27-nvic")).unwrap()
        })
    }

    fn symbol(name: &str) -> u32 {
        let parsed = elf::parse(firmware());
        let s = parsed.symbols.iter().find(|s| s.name == name);
        s.unwrap_or_else(|| panic!("нет символа {name}")).value as u32
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_a_table_matches_symbols() {
        let v = vector_table(firmware()).unwrap();
        assert_eq!(v.addr, 0, "таблица — с адреса 0");
        assert_eq!(v.check(), Ok(()));
        assert_eq!(v.get(0), Some(symbol("_stack_start")), "слово 0 — начальный SP");
        // Адреса функций Thumb в таблице символов уже с битом 0.
        assert_eq!(v.get(1), Some(symbol("Reset")));
        assert_eq!(v.get(3), Some(symbol("HardFault")));
        assert_eq!(v.get(11), Some(symbol("SVCall")));
        assert_eq!(v.get(15), Some(symbol("DefaultHandler")), "своего SysTick нет");
        for (irq, name) in [(0, "GPIOA"), (1, "GPIOB"), (2, "GPIOC"), (3, "DefaultHandler")] {
            assert_eq!(v.get(16 + irq), Some(symbol(name)), "прерывание {irq}");
        }
        // Зарезервированные слова 7–10 и 13 — нули.
        for i in [7, 8, 9, 10, 13] {
            assert_eq!(v.get(i), Some(0), "слово {i}");
        }
        let parsed = elf::parse(firmware());
        let interrupts = parsed.symbols.iter().find(|s| s.name == "__INTERRUPTS").unwrap();
        assert_eq!(v.interrupts(), interrupts.size as usize / 4);
        assert_eq!(v.get(v.len()), None);
    }

    /// Смещение таблицы в файле: первые два слова — SP и Reset — в файле
    /// встречаются один раз.
    fn table_offset(elf: &[u8]) -> usize {
        let v = vector_table(elf).unwrap();
        let head: Vec<u8> = [v.get(0).unwrap(), v.get(1).unwrap()]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        elf.windows(8).position(|w| w == head).unwrap()
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_a_broken_tables() {
        let good = firmware();
        let at = table_offset(good);

        let mut even = good.to_vec();
        even[at + 3 * 4] &= !1;
        let addr = symbol("HardFault") & !1;
        assert_eq!(
            vector_table(&even).unwrap().check(),
            Err(VtError::EvenHandler { index: 3, addr })
        );

        let mut sp = good.to_vec();
        sp[at] = sp[at].wrapping_sub(4);
        let bad_sp = u32::from_le_bytes(sp[at..at + 4].try_into().unwrap());
        assert_eq!(vector_table(&sp).unwrap().check(), Err(VtError::StackNotAligned(bad_sp)));

        let mut renamed = good.to_vec();
        let name = b".vector_table\0";
        let i = renamed.windows(name.len()).position(|w| w == name).unwrap();
        renamed[i + 7] = b'X';
        assert_eq!(vector_table(&renamed).err(), Some(VtError::NoTable));

        assert_eq!(vector_table(&good[..0x40]).err(), Some(VtError::Truncated));
    }

    #[test]
    fn a_not_elf32() {
        assert_eq!(vector_table(b"hello, world").err(), Some(VtError::NotElf32));
        let mut elf64 = b"\x7fELF\x02\x01".to_vec();
        elf64.resize(64, 0);
        assert_eq!(vector_table(&elf64).err(), Some(VtError::NotElf32));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;

    /// Ассемблер крейта задач под Cortex-M3, без LTO.
    fn asm() -> &'static str {
        static ASM: OnceLock<String> = OnceLock::new();
        ASM.get_or_init(|| {
            let target = "thumbv7m-none-eabi";
            let out = tmp("asm");
            let pkg = env!("CARGO_PKG_NAME");
            let res = pinned("cargo", Path::new(env!("CARGO_MANIFEST_DIR")))
                .args(["rustc", "-p", pkg, "--lib", "--release", "--target", target])
                .arg("--target-dir")
                .arg(&out)
                .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
                .output()
                .unwrap();
            assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
            let d = out.join(target).join("release/deps");
            let prefix = format!("{}-", pkg.replace('-', "_"));
            let file = std::fs::read_dir(&d)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| {
                    let n = p.file_name().unwrap().to_string_lossy();
                    n.starts_with(&prefix) && n.ends_with(".s")
                })
                .max_by_key(|p| p.metadata().unwrap().modified().unwrap())
                .unwrap_or_else(|| panic!("нет файла .s в {}", d.display()));
            std::fs::read_to_string(file).unwrap()
        })
    }

    /// Мнемоники функции без операндов; склеенную функцию (`a = b`)
    /// заменяет тело `b`.
    fn ops(name: &str) -> Vec<String> {
        let asm = asm();
        if let Some(other) = asm.lines().find_map(|l| l.strip_prefix(&format!("{name} = "))) {
            return ops(other.trim());
        }
        let label = format!("\n{name}:\n");
        let start = asm.find(&label).unwrap_or_else(|| panic!("нет {name}"));
        let rest = &asm[start + label.len()..];
        let end = rest.find(".Lfunc_end").unwrap();
        rest[..end]
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with(['.', '@', '#']))
            .map(|l| l.split_whitespace().next().unwrap().to_owned())
            .collect()
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn rustc_b_three_ways_one_code() {
        let raw = ops("ch27_b_raw");
        for name in ["ch27_b_pac", "ch27_b_hal"] {
            assert_eq!(ops(name), raw, "{name} отличается от ch27_b_raw");
        }
        // Вызовов нет: HAL и PAC встроились целиком.
        assert!(!raw.iter().any(|op| op == "bl" || op == "blx"), "вызовы: {raw:?}");
        let stores = raw.iter().filter(|op| op.starts_with("str")).count();
        let loads = raw.iter().filter(|op| op.starts_with("ldr")).count();
        assert_eq!(stores, 1, "одна запись в DR: {raw:?}");
        assert!(loads >= 1, "чтение FR в цикле: {raw:?}");
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::sut::c_lines::{CmdError, Command, Feed, LineEditor, Sink, parse};

    #[derive(Default)]
    struct Out(Vec<u8>);

    impl Sink for Out {
        fn put(&mut self, bytes: &[u8]) {
            self.0.extend_from_slice(bytes);
        }
    }

    /// Прогнать байты через редактор: готовые строки (`None` — слишком
    /// длинная) и всё эхо.
    fn run<const N: usize>(input: &[u8]) -> (Vec<Option<String>>, Vec<u8>) {
        let mut ed = LineEditor::<N>::new();
        let mut out = Out::default();
        let mut lines = Vec::new();
        for &b in input {
            match ed.feed(b, &mut out) {
                Feed::Pending => {}
                Feed::Line(s) => lines.push(Some(s.to_owned())),
                Feed::TooLong => lines.push(None),
            }
        }
        (lines, out.0)
    }

    #[test]
    fn c_lines_and_echo() {
        let (lines, echo) = run::<16>(b"status\rperiod 50\n");
        assert_eq!(lines, [Some("status".into()), Some("period 50".into())]);
        assert_eq!(echo, b"status\r\nperiod 50\r\n");
        // \r\n — одна строка, пустые строки не выдаются.
        let (lines, _) = run::<16>(b"stop\r\n\r\r\nstart\r\n");
        assert_eq!(lines, [Some("stop".into()), Some("start".into())]);
    }

    #[test]
    fn c_backspace() {
        let (lines, echo) = run::<16>(b"stx\x08art\r");
        assert_eq!(lines, [Some("start".into())]);
        assert_eq!(echo, b"stx\x08 \x08art\r\n");
        // DEL — то же; в пустой строке забой ничего не делает.
        let (lines, echo) = run::<16>(b"\x7f\x08ab\x7f\x7f\x7fc\r");
        assert_eq!(lines, [Some("c".into())]);
        assert_eq!(echo, b"ab\x08 \x08\x08 \x08c\r\n");
    }

    #[test]
    fn c_too_long_and_garbage() {
        // Девятый символ не помещается: звонок один раз, строка отброшена.
        let (lines, echo) = run::<8>(b"abcdefghij\rok\r");
        assert_eq!(lines, [None, Some("ok".into())]);
        assert_eq!(echo, b"abcdefgh\x07\r\nok\r\n");
        // Восемь символов — ровно по буферу.
        let (lines, _) = run::<8>(b"abcdefgh\r");
        assert_eq!(lines, [Some("abcdefgh".into())]);
        // Управляющие и не-ASCII байты пропускаются без эха.
        let (lines, echo) = run::<8>(b"s\x1b\x00t\xd0\xbfop\r");
        assert_eq!(lines, [Some("stop".into())]);
        assert_eq!(echo, b"stop\r\n");
    }

    #[test]
    fn c_parse() {
        use CmdError::*;
        let cases: &[(&str, Result<Command, CmdError>)] = &[
            ("status", Ok(Command::Status)),
            ("  start  ", Ok(Command::Start)),
            ("stop", Ok(Command::Stop)),
            ("period 10", Ok(Command::Period(10))),
            ("period   10000", Ok(Command::Period(10_000))),
            ("", Err(Empty)),
            ("   ", Err(Empty)),
            ("Status", Err(Unknown)),
            ("reboot", Err(Unknown)),
            ("period", Err(MissingArg)),
            ("period 9", Err(OutOfRange)),
            ("period 10001", Err(OutOfRange)),
            ("period +50", Err(BadNumber)),
            ("period -5", Err(BadNumber)),
            ("period 5x", Err(BadNumber)),
            ("period 99999999999", Err(BadNumber)),
            ("period 50 60", Err(Extra)),
            ("stop now", Err(Extra)),
        ];
        for (line, want) in cases {
            assert_eq!(parse(line), *want, "{line:?}");
        }
        assert_eq!(OutOfRange.message(), "период вне 10..10000 мс");
    }
}

// --------------------------------------------------------------- задача D1

mod task_d1 {
    use super::sut::d1_switch::{FRAME_WORDS, NO_RETURN, RoundRobin, Stack, XPSR_T};

    #[test]
    fn d1_initial_frame() {
        let mut s = Stack::<64>::new();
        let base = s.words.as_ptr() as usize;
        let psp = s.prepare(0x0000_1235, 0xA5) as usize;
        assert_eq!((psp - base) / 4, 64 - FRAME_WORDS, "кадр — у вершины стека");
        assert_eq!(psp % 8, 0, "PSP выровнен на 8");
        let f = &s.words[64 - FRAME_WORDS..];
        // r4–r11 — их номера.
        let regs: Vec<u32> = (4..=11).map(|r| 0x0101_0101 * r).collect();
        assert_eq!(f[..8], regs[..]);
        // r0 — аргумент, r1–r3 и r12 — нули, lr — сбой при возврате, pc без
        // бита 0, xPSR — только T.
        assert_eq!(f[8..], [0xA5, 0, 0, 0, 0, NO_RETURN, 0x0000_1234, XPSR_T]);
        // Остальной стек не тронут.
        assert!(s.words[..64 - FRAME_WORDS].iter().all(|&w| w == 0));
    }

    #[test]
    fn d1_stack_alignment() {
        assert_eq!(align_of::<Stack<16>>(), 8);
        assert_eq!(size_of::<Stack<256>>(), 1024);
    }

    #[test]
    fn d1_round_robin() {
        let mut q = RoundRobin::<3>::new();
        assert_eq!(q.current(), None);
        assert_eq!(q.add(0x100), Ok(0));
        assert_eq!(q.add(0x200), Ok(1));
        assert_eq!(q.add(0x300), Ok(2));
        assert_eq!(q.add(0x400), Err(0x400));
        // Первое переключение: уходящей задачи нет, значение отбрасывается.
        assert_eq!(q.switch(0xDEAD), 0x100);
        assert_eq!(q.current(), Some(0));
        // Дальше каждая задача уходит со своим PSP и получает его обратно.
        assert_eq!(q.switch(0x0F0), 0x200);
        assert_eq!(q.switch(0x1F0), 0x300);
        assert_eq!(q.switch(0x2F0), 0x0F0);
        assert_eq!(q.switch(0x0E0), 0x1F0);
        assert_eq!(q.current(), Some(1));
    }

    #[test]
    #[should_panic(expected = "нет задач")]
    fn d1_no_tasks() {
        RoundRobin::<2>::new().switch(0);
    }
}

// --------------------------------------------------------------- задача D2

mod task_d2 {
    use super::sut::d2_faults::{Fault, faults, insn_len};

    fn names(cfsr: u32, hfsr: u32) -> Vec<&'static str> {
        faults(cfsr, hfsr).map(Fault::name).collect()
    }

    #[test]
    fn d2_known_values() {
        // Значения из прошивок главы.
        assert_eq!(names(0x0002_0000, 0x4000_0000), ["UFSR.INVSTATE", "HFSR.FORCED"]);
        assert_eq!(names(0x0000_0092, 0), ["MMFSR.DACCVIOL", "MMFSR.MSTKERR", "MMFSR.MMARVALID"]);
        assert_eq!(names(0x0001_0000, 0), ["UFSR.UNDEFINSTR"]);
        assert_eq!(names(0x0200_0000, 0), ["UFSR.DIVBYZERO"]);
        assert_eq!(names(0x0100_0000, 0), ["UFSR.UNALIGNED"]);
        assert_eq!(names(0x0008_0000, 0), ["UFSR.NOCP"]);
        assert_eq!(names(0x0000_8200, 0), ["BFSR.PRECISERR", "BFSR.BFARVALID"]);
        assert_eq!(names(0, 0x0000_0002), ["HFSR.VECTTBL"]);
        assert_eq!(names(0, 0x8000_0000), ["HFSR.DEBUGEVT"]);
        assert!(names(0, 0).is_empty());
    }

    #[test]
    fn d2_every_bit_once() {
        // 19 битов CFSR и 3 HFSR, у каждого своё имя; зарезервированные
        // биты не дают ничего.
        let all = names(u32::MAX, u32::MAX);
        assert_eq!(all.len(), 22);
        let mut unique = all.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 22);
        let reserved_cfsr = !0x030F_BFBB_u32;
        let reserved_hfsr = !0xC000_0002_u32;
        assert!(names(reserved_cfsr, reserved_hfsr).is_empty());
        for bit in 0..32 {
            let n = names(1 << bit, 0).len();
            assert_eq!(n, usize::from(reserved_cfsr & 1 << bit == 0), "CFSR бит {bit}");
        }
    }

    #[test]
    fn d2_instruction_length() {
        assert_eq!(insn_len(0xDE2A), 2, "udf #42");
        assert_eq!(insn_len(0xBF00), 2, "nop");
        assert_eq!(insn_len(0x4770), 2, "bx lr");
        assert_eq!(insn_len(0xFBB0), 4, "udiv");
        assert_eq!(insn_len(0xF7FF), 4, "bl");
        assert_eq!(insn_len(0xE92D), 4, "push.w");
        assert_eq!(insn_len(0xE7FE), 2, "b . — 16-битная, хотя начинается с 0b11100");
    }
}
