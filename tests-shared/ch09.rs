// Общие тесты задач главы 9.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch09/tests/ch09.rs` и `solutions/ch09/tests/ch09.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задачи A, B и C собирают пробы закреплённым rustc, задача B — ещё и сам
// крейт под Cortex-M0. Под Miri эти тесты пропускаются; остальные —
// обычный код и под Miri идут.

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
        .env("CARGO_TERM_COLOR", "never");
    cmd
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

/// Собрать `file` закреплённым rustc в каталоге `dir`. Ошибки — в коротком
/// формате, по строке на ошибку. Вернуть успех и текст ошибок.
fn rustc(dir: &Path, file: &Path, name: &str, args: &[&str]) -> (bool, String) {
    let out = pinned("rustc", crate_dir())
        .args(["--edition", "2024", "--error-format", "short", "-A", "warnings"])
        .args(["--crate-name", name, "--out-dir"])
        .arg(dir)
        .args(args)
        .arg(file)
        .output()
        .expect("не удалось запустить rustc");
    (out.status.success(), text(&out.stderr))
}

/// Коды ошибок из вывода rustc: `error[E0277]` → `E0277`.
fn codes(err: &str) -> Vec<&str> {
    err.match_indices("error[E").map(|(i, _)| &err[i + 6..i + 11]).collect()
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use sut::a_coherence::{Verdict, verdicts};

    /// Что на деле говорит компилятор о фрагменте `a{i}.rs`.
    fn verdict(i: usize) -> Verdict {
        let dir = crate_dir().join("../../code/ch09/fragments");
        let file = dir.join(format!("a{i}.rs"));
        let src = std::fs::read_to_string(&file).unwrap();
        let (ok, err) = rustc(
            &tmp("a"),
            &file,
            &format!("a{i}"),
            &["--crate-type", "lib", "--emit", "metadata"],
        );
        if ok {
            return Verdict::Compiles;
        }
        // Строки с метками ⟨n⟩: ошибка относится к последней метке выше неё.
        let marks: Vec<(usize, u8)> = src
            .lines()
            .enumerate()
            .filter_map(|(at, l)| {
                let n = l.trim().strip_prefix("// ⟨")?.strip_suffix('⟩')?.parse().ok()?;
                Some((at + 1, n))
            })
            .collect();
        let mut orphan = None;
        let mut overlap = false;
        for line in err.lines().filter(|l| l.contains(": error[E")) {
            // a1.rs:11:1: error[E0117]: …
            let place = &line[..line.find(": error[E").unwrap()];
            let at: usize = place.rsplit(':').nth(1).unwrap().parse().unwrap();
            match codes(line)[0] {
                "E0117" | "E0210" => {
                    let n = marks.iter().rev().find(|(m, _)| *m < at).expect("ошибка выше меток").1;
                    orphan.get_or_insert(n);
                }
                "E0119" => overlap = true,
                _ => panic!("a{i}: ошибка не о когерентности:\n{err}"),
            }
        }
        match (orphan, overlap) {
            (Some(n), _) => Verdict::Orphan(n),
            (None, true) => Verdict::Overlap,
            (None, false) => panic!("a{i}: не собирается, но кодов ошибок нет:\n{err}"),
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_a_coherence() {
        let answers = verdicts();
        let actual: Vec<Verdict> = std::thread::scope(|s| {
            let jobs: Vec<_> = (1..=6).map(|i| s.spawn(move || verdict(i))).collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });
        let wrong: Vec<String> = (0..6)
            .filter(|&i| answers[i] != actual[i])
            .map(|i| {
                format!("a{}: ответ {:?}, а компилятор говорит {:?}", i + 1, answers[i], actual[i])
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;
    use std::collections::{HashMap, VecDeque};
    use sut::b_register::{BadBaud, Bus, Cr, Fr, Icr, Reg, deinit, init, write_byte};

    const DR: usize = 0x4000_C000;
    const FR: usize = 0x4000_C018;
    const IBRD: usize = 0x4000_C024;
    const FBRD: usize = 0x4000_C028;
    const LCRH: usize = 0x4000_C02C;
    const CR: usize = 0x4000_C030;
    const ICR: usize = 0x4000_C044;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Op {
        R(usize),
        W(usize, u32),
    }

    /// Шина-журнал: помнит записанное и отдаёт FR из очереди.
    struct FakeBus {
        log: Vec<Op>,
        regs: HashMap<usize, u32>,
        fr: VecDeque<u32>,
    }

    impl FakeBus {
        fn new() -> FakeBus {
            // Значения после сброса: FIFO пусты (TXFE, RXFE), TXE и RXE.
            let regs = HashMap::from([(FR, 0x90), (CR, 0x300)]);
            FakeBus { log: Vec::new(), regs, fr: VecDeque::new() }
        }

        fn writes(&self) -> Vec<(usize, u32)> {
            self.log
                .iter()
                .filter_map(|op| match *op {
                    Op::W(a, v) => Some((a, v)),
                    Op::R(_) => None,
                })
                .collect()
        }
    }

    impl Bus for FakeBus {
        unsafe fn read(&mut self, addr: usize) -> u32 {
            self.log.push(Op::R(addr));
            match self.fr.pop_front() {
                Some(v) if addr == FR => v,
                other => {
                    if let Some(v) = other {
                        self.fr.push_front(v);
                    }
                    self.regs.get(&addr).copied().unwrap_or(0)
                }
            }
        }

        unsafe fn write(&mut self, addr: usize, value: u32) {
            self.log.push(Op::W(addr, value));
            self.regs.insert(addr, value);
        }
    }

    #[test]
    fn task_b_init() {
        let mut bus = FakeBus::new();
        assert_eq!(init(&mut bus, 12_000_000, 115_200), Ok(()));
        assert_eq!(bus.writes(), [(CR, 0), (IBRD, 6), (FBRD, 33), (LCRH, 0x70), (CR, 0x301)]);
    }

    #[test]
    fn task_b_baud() {
        // (частота, скорость) → (IBRD, FBRD) или ошибка.
        let cases = [
            (12_000_000, 115_200, Some((6, 33))),
            (12_000_000, 9_600, Some((78, 8))),
            (50_000_000, 115_200, Some((27, 8))),
            (16_000_000, 9_600, Some((104, 11))),
            (12_000_000, 750_000, Some((1, 0))),
            (12_000_000, 12, Some((62_500, 0))),
            (4_194_240, 4, Some((0xffff, 0))),
            (u32::MAX, 115_200, Some((2330, 11))),
            (12_000_000, 1_000_000, None),
            (12_000_000, 11, None),
            (12_000_000, 0, None),
            (4_194_241, 4, None),
            (u32::MAX, 1, None),
            (1, u32::MAX, None),
        ];
        for (clock, baud, want) in cases {
            let mut bus = FakeBus::new();
            let got = init(&mut bus, clock, baud);
            match want {
                Some((i, f)) => {
                    assert_eq!(got, Ok(()), "{clock} Гц, {baud} бод");
                    assert_eq!(
                        bus.writes(),
                        [(CR, 0), (IBRD, i), (FBRD, f), (LCRH, 0x70), (CR, 0x301)],
                        "{clock} Гц, {baud} бод"
                    );
                }
                None => {
                    assert_eq!(got, Err(BadBaud), "{clock} Гц, {baud} бод");
                    assert!(
                        bus.log.is_empty(),
                        "{clock} Гц, {baud} бод: при ошибке трогать UART нельзя"
                    );
                }
            }
        }
    }

    #[test]
    fn task_b_write_byte() {
        let mut bus = FakeBus::new();
        // Трижды FIFO полон (TXFF и BUSY), потом место есть.
        bus.fr.extend([0x28, 0x28, 0x28]);
        write_byte(&mut bus, b'A');
        assert_eq!(bus.log, [Op::R(FR), Op::R(FR), Op::R(FR), Op::R(FR), Op::W(DR, 0x41)]);
    }

    #[test]
    fn task_b_reg() {
        let mut bus = FakeBus::new();
        bus.regs.insert(CR, 0x301);
        Reg::<Cr>::modify(&mut bus, |v| v & !1);
        assert_eq!(Reg::<Fr>::read(&mut bus), 0x90);
        Reg::<Icr>::write(&mut bus, 0x7ff);
        assert_eq!(bus.log, [Op::R(CR), Op::W(CR, 0x300), Op::R(FR), Op::W(ICR, 0x7ff)]);
    }

    #[test]
    fn task_b_reset() {
        let mut bus = FakeBus::new();
        assert!(Reg::<Fr>::is_reset(&mut bus));
        bus.fr.push_back(0x98); // BUSY: передатчик занят
        assert!(!Reg::<Fr>::is_reset(&mut bus));
        init(&mut bus, 12_000_000, 115_200).unwrap();
        assert!(!Reg::<Cr>::is_reset(&mut bus));
        bus.log.clear();
        deinit(&mut bus);
        assert_eq!(bus.writes(), [(CR, 0x300), (IBRD, 0), (FBRD, 0), (LCRH, 0)]);
        assert!(Reg::<Cr>::is_reset(&mut bus));
    }

    /// Регистр, читается ли он и пишется ли.
    const ACCESS: [(&str, bool, bool); 7] = [
        ("Dr", true, true),
        ("Fr", true, false),
        ("Ibrd", true, true),
        ("Fbrd", true, true),
        ("Lcrh", true, true),
        ("Cr", true, true),
        ("Icr", false, true),
    ];

    fn probe(method: &str, reg: &str) -> String {
        match method {
            "read" => format!("pub fn probe(b: &mut Mmio) -> u32 {{ Reg::<{reg}>::read(b) }}"),
            "is_reset" => {
                format!("pub fn probe(b: &mut Mmio) -> bool {{ Reg::<{reg}>::is_reset(b) }}")
            }
            "write" => format!("pub fn probe(b: &mut Mmio) {{ Reg::<{reg}>::write(b, 0) }}"),
            "reset" => format!("pub fn probe(b: &mut Mmio) {{ Reg::<{reg}>::reset(b) }}"),
            _ => format!("pub fn probe(b: &mut Mmio) {{ Reg::<{reg}>::modify(b, |v| v) }}"),
        }
    }

    /// Свой регистр по адресу BASE + offset.
    fn own(offset: usize) -> String {
        format!(
            "pub struct Own;\n\
             unsafe impl Register for Own {{\n\
                 const ADDR: usize = BASE + {offset};\n\
                 const RESET: u32 = 0;\n\
             }}\n\
             impl Readable for Own {{}}\n\
             impl Writable for Own {{}}\n"
        )
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_b_access() {
        let src = std::fs::read_to_string(crate_dir().join("src/b_register.rs")).unwrap();
        let dir = tmp("b-probes");
        // (что проверяем, код пробы, должна ли собраться, --emit)
        let mut probes = Vec::new();
        for (reg, r, w) in ACCESS {
            let methods =
                [("read", r), ("is_reset", r), ("write", w), ("reset", w), ("modify", r && w)];
            for (method, ok) in methods {
                probes.push((format!("{reg}::{method}"), probe(method, reg), ok, "metadata"));
            }
        }
        // Выровненный и невыровненный адреса проверяются при генерации
        // кода, поэтому эти пробы собираются в объектный файл.
        for method in ["read", "is_reset", "write", "reset", "modify"] {
            let good = own(4) + &probe(method, "Own");
            let bad = own(2) + &probe(method, "Own");
            probes.push((format!("BASE + 4, {method}"), good, true, "obj"));
            probes.push((format!("BASE + 2, {method}"), bad, false, "obj"));
        }
        let results: Vec<(bool, String)> = std::thread::scope(|s| {
            let jobs: Vec<_> = probes
                .iter()
                .enumerate()
                .map(|(i, (_, code, _, emit))| {
                    let (src, dir) = (&src, &dir);
                    s.spawn(move || {
                        let file = dir.join(format!("b{i}.rs"));
                        std::fs::write(&file, format!("{src}\n{code}\n")).unwrap();
                        let emit = format!("--emit={emit}");
                        rustc(dir, &file, &format!("b{i}"), &["--crate-type", "lib", &emit])
                    })
                })
                .collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });
        let mut wrong = Vec::new();
        for ((what, _, want, _), (ok, err)) in probes.iter().zip(&results) {
            if ok != want {
                wrong.push(format!(
                    "{what}: {}",
                    if *ok {
                        "собирается, а не должно"
                    } else {
                        "не собирается"
                    }
                ));
                if !ok {
                    wrong.push(err.clone());
                }
            } else if !ok {
                // Сломаться проба должна на ограничениях или на проверке
                // выравнивания, а не на чём-то ещё.
                let expected: &[&str] =
                    if what.starts_with("BASE") { &["E0080"] } else { &["E0599", "E0277"] };
                let codes = codes(err);
                if codes.is_empty() || !codes.iter().all(|c| expected.contains(c)) {
                    wrong.push(format!("{what}: ждали {expected:?}, а вышло\n{err}"));
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// Минимальный разбор ELF32 little-endian: секции с содержимым и
    /// символы. Хватает для объектных файлов Cortex-M.
    mod elf {
        pub struct Elf<'a> {
            /// Имя секции и её байты (у NOBITS — пусто).
            pub sections: Vec<(String, &'a [u8])>,
            /// Имя символа, номер секции (0 — не определён) и глобальность.
            pub symbols: Vec<(String, usize, bool)>,
        }

        fn u(b: &[u8], at: usize, size: usize) -> usize {
            b[at..at + size].iter().rev().fold(0, |v, &x| v << 8 | usize::from(x))
        }

        fn cstr(b: &[u8], at: usize) -> String {
            let end = b[at..].iter().position(|&c| c == 0).unwrap();
            String::from_utf8_lossy(&b[at..at + end]).into_owned()
        }

        pub fn parse(b: &[u8]) -> Elf<'_> {
            assert!(b.starts_with(b"\x7fELF\x01\x01"), "ожидался ELF32 little-endian");
            let (shoff, shentsize, shnum, shstrndx) =
                (u(b, 0x20, 4), u(b, 0x2e, 2), u(b, 0x30, 2), u(b, 0x32, 2));
            let sh = |i: usize, f: usize| u(b, shoff + i * shentsize + f, 4);
            let names = sh(shstrndx, 16);
            let sections = (0..shnum)
                .map(|i| {
                    let (ty, off, size) = (sh(i, 4), sh(i, 16), sh(i, 20));
                    let data = if ty == 8 { &b[0..0] } else { &b[off..off + size] };
                    (cstr(b, names + sh(i, 0)), data)
                })
                .collect();
            let mut symbols = Vec::new();
            for i in (0..shnum).filter(|&i| sh(i, 4) == 2) {
                let strtab = sh(sh(i, 24), 16);
                for s in (sh(i, 16)..sh(i, 16) + sh(i, 20)).step_by(16).skip(1) {
                    symbols.push((
                        cstr(b, strtab + u(b, s, 4)),
                        u(b, s + 14, 2),
                        b[s + 12] >> 4 != 0,
                    ));
                }
            }
            Elf { sections, symbols }
        }

        /// Члены архива ar в формате GNU, кроме служебных.
        pub fn ar(b: &[u8]) -> Vec<&[u8]> {
            assert!(b.starts_with(b"!<arch>\n"), "не архив ar");
            let mut pos = 8;
            let mut out = Vec::new();
            while pos + 60 <= b.len() {
                let size: usize =
                    String::from_utf8_lossy(&b[pos + 48..pos + 58]).trim().parse().unwrap();
                out.push(&b[pos + 60..pos + 60 + size]);
                pos += 60 + size + size % 2;
            }
            out
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_b_thumb() {
        let target = tmp("b-thumb");
        // cargo через прокси rustup, а не env!("CARGO"): прокси прочтёт
        // rust-toolchain.toml и передаст выбор дальше. Иначе embedded-hal,
        // которую cargo компилирует из её каталога в реестре, где
        // rust-toolchain.toml нет, собрал бы тулчейн по умолчанию.
        let out = pinned("cargo", crate_dir())
            .args(["build", "--lib", "--release", "--target", "thumbv6m-none-eabi", "--target-dir"])
            .arg(&target)
            .output()
            .unwrap();
        assert!(out.status.success(), "сборка под Cortex-M0:\n{}", text(&out.stderr));
        let lib = format!("lib{}.rlib", env!("CARGO_PKG_NAME").replace('-', "_"));
        let bytes = std::fs::read(target.join("thumbv6m-none-eabi/release").join(lib)).unwrap();

        let mut defined = Vec::new();
        let (mut in_code, mut in_data) = (Vec::new(), Vec::new());
        for member in elf::ar(&bytes).into_iter().filter(|m| m.starts_with(b"\x7fELF")) {
            let obj = elf::parse(member);
            for (name, shndx, global) in &obj.symbols {
                if name.starts_with("ch09b_") && *shndx != 0 && *global {
                    defined.push(name.clone());
                }
            }
            for (name, data) in &obj.sections {
                // Слова, похожие на адреса регистров UART0.
                let words = data
                    .chunks_exact(4)
                    .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
                    .filter(|w| (0x4000_C000..=0x4000_C044).contains(w));
                if name.starts_with(".text") {
                    in_code.extend(words);
                } else if name.starts_with(".rodata") || name.starts_with(".data") {
                    in_data.extend(words.map(|w| (name.clone(), w)));
                }
            }
        }
        defined.sort();
        assert_eq!(defined, ["ch09b_init_115200", "ch09b_write_byte"]);
        assert!(!in_code.is_empty(), "адресов регистров нет в коде: их должен знать компилятор");
        assert!(in_data.is_empty(), "адреса регистров лежат в данных: {in_data:x?}");
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use sut::c_sealed::*;

    macro_rules! pins {
        ($($p:ident $port:literal $n:literal),*) => {
            $(
                assert_eq!(
                    (<$p as PinId>::PORT, <$p as PinId>::N, <$p as PinId>::MASK),
                    ($port, $n, 1u32 << $n),
                    stringify!($p)
                );
            )*
        };
    }

    #[test]
    fn task_c_pins() {
        pins!(PA0 0 0, PA1 0 1, PA2 0 2, PA3 0 3, PA4 0 4, PA5 0 5, PA6 0 6, PA7 0 7);
        pins!(PB0 1 0, PB1 1 1, PB2 1 2, PB3 1 3, PB4 1 4, PB5 1 5, PB6 1 6, PB7 1 7);
        assert_eq!(bsrr::<PA3>(true), 0x0000_0008);
        assert_eq!(bsrr::<PA3>(false), 0x0008_0000);
        assert_eq!(bsrr::<PB7>(false), 0x0080_0000);
        assert_eq!(bsrr::<PB0>(true), 0x0000_0001);
        assert_eq!(bsrr_addr::<PA0>(), 0x4001_0810);
        assert_eq!(bsrr_addr::<PB5>(), 0x4001_0C10);
    }

    #[test]
    fn task_c_bits() {
        let x = 0xdead_beef_u32;
        assert!(x.bit(0) && x.bit(28) && x.bit(31) && !x.bit(4) && !x.bit(29));
        assert_eq!(0u32.with_bit(31, true), 0x8000_0000);
        assert_eq!(u32::MAX.with_bit(3, false), 0xffff_fff7);
        assert_eq!(8u32.with_bit(3, true), 8);
        assert_eq!(0x70u32.field(5, 2), 0b11);
        assert_eq!(x.field(28, 4), 0xd);
        assert_eq!(x.field(4, 8), 0xee);
        assert_eq!(x.field(31, 1), 1);
        assert_eq!(x.field(0, 32), x);
        assert_eq!(0u32.with_field(4, 4, 0xb), 0xb0);
        assert_eq!(u32::MAX.with_field(8, 8, 0), 0xffff_00ff);
        assert_eq!(x.with_field(0, 32, 0x1234_5678), 0x1234_5678);
        assert_eq!(x.with_field(16, 16, 0xcafe), 0xcafe_beef);
        assert_eq!(0x300u32.with_field(5, 2, 3).with_bit(0, true), 0x361);
    }

    /// Вызов, который должен паниковать, и его запись для сообщения.
    type Case = (&'static str, fn() -> u32);

    #[test]
    fn task_c_bits_panic() {
        let cases: [Case; 8] = [
            ("bit(32)", || u32::from(1u32.bit(32))),
            ("with_bit(32, _)", || 1u32.with_bit(32, true)),
            ("field(0, 0)", || 1u32.field(0, 0)),
            ("field(16, 17)", || 1u32.field(16, 17)),
            ("field(32, 1)", || 1u32.field(32, 1)),
            ("field(u32::MAX, 1)", || 1u32.field(u32::MAX, 1)),
            ("with_field(4, 4, 0x10)", || 1u32.with_field(4, 4, 0x10)),
            ("with_field(0, 33, 0)", || 1u32.with_field(0, 33, 0)),
        ];
        for (what, f) in cases {
            let payload = match std::panic::catch_unwind(f) {
                Ok(v) => panic!("{what} должно паниковать, а вернуло {v:#x}"),
                Err(p) => p,
            };
            let msg = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("");
            assert!(
                !msg.starts_with("not yet implemented"),
                "{what}: паника из todo!() не считается"
            );
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_c_sealed() {
        let dir = tmp("c");
        let (ok, err) = rustc(
            &dir,
            &crate_dir().join("src/c_sealed.rs"),
            "pins",
            &["--crate-type", "rlib", "--emit", "metadata"],
        );
        assert!(ok, "c_sealed.rs отдельно не собирается:\n{err}");
        let pins = format!("pins={}", dir.join("libpins.rmeta").display());
        let user = |name: &str, code: &str| {
            let file = dir.join(format!("{name}.rs"));
            std::fs::write(&file, code).unwrap();
            rustc(
                &dir,
                &file,
                name,
                &["--crate-type", "lib", "--emit", "metadata", "--extern", &pins],
            )
        };

        // Пользоваться выводами и расширением можно.
        let (ok, err) = user(
            "c_use",
            "use pins::{PinId, U32Ext, PA3, PB7};\n\
             pub fn a() -> u32 { PA3::MASK | PB7::MASK }\n\
             pub fn b<P: PinId>() -> u8 { P::PORT * 16 + P::N }\n\
             pub fn c(x: u32) -> u32 { x.with_field(4, 4, 0xb).with_bit(0, x.bit(1)) }\n\
             pub fn d() -> (u32, usize) { (pins::bsrr::<PA3>(true), pins::bsrr_addr::<PA3>()) }\n",
        );
        assert!(ok, "пользоваться pins из другого крейта не получается:\n{err}");

        // Добавить свой вывод нельзя.
        let (ok, err) = user(
            "c_pin",
            "pub struct PC0;\nimpl pins::PinId for PC0 { const PORT: u8 = 2; const N: u8 = 0; }\n",
        );
        assert!(!ok, "другой крейт реализовал PinId: трейт не запечатан");
        assert!(codes(&err).contains(&"E0277"), "ждали E0277 от недостающего супертрейта:\n{err}");
        // Сама печать: `PC0: pins::…::Sealed`.
        let seal = err
            .split("`PC0: ")
            .nth(1)
            .and_then(|s| s.split('`').next())
            .filter(|s| s.starts_with("pins::") && *s != "pins::PinId")
            .unwrap_or_else(|| panic!("не видно печати в ошибке:\n{err}"));
        let (ok, err) = user("c_seal", &format!("pub struct PC0;\nimpl {seal} for PC0 {{}}\n"));
        assert!(!ok, "печать {seal} реализуется из другого крейта");
        assert!(
            codes(&err).contains(&"E0603"),
            "печать должна быть недоступна по пути (E0603):\n{err}"
        );

        // Расширение для своего типа тоже не реализовать. Для u16 не
        // понадобилась бы и печать: чужой трейт для чужого типа — E0117.
        let (ok, err) = user(
            "c_ext",
            "pub struct Raw(pub u32);\nimpl pins::U32Ext for Raw {\n\
             fn bit(self, _: u32) -> bool { false }\n\
             fn with_bit(self, _: u32, _: bool) -> Raw { self }\n\
             fn field(self, _: u32, _: u32) -> u32 { 0 }\n\
             fn with_field(self, _: u32, _: u32, _: u32) -> Raw { self }\n}\n",
        );
        assert!(!ok, "другой крейт реализовал U32Ext для своего типа: трейт не запечатан");
        assert!(codes(&err).contains(&"E0277"), "ждали E0277 от недостающего супертрейта:\n{err}");

        // Метод расширения без `use` не виден.
        let (ok, err) = user("c_nouse", "pub fn f(x: u32) -> bool { x.bit(3) }\n");
        assert!(!ok && codes(&err) == ["E0599"], "ждали E0599 без use pins::U32Ext:\n{err}");
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;
    use embedded_hal::i2c::{self, I2c, NoAcknowledgeSource};
    use embedded_hal::spi::{self, SpiDevice};
    use sut::d_bus::{Error, Tmp9};

    fn hex(data: &[u8]) -> String {
        let bytes: Vec<String> = data.iter().map(|b| format!("{b:02x}")).collect();
        format!("[{}]", bytes.join(" "))
    }

    /// Модель TMP-9: регистры и готовность измерения.
    struct Chip {
        id: u8,
        ctrl: u8,
        temp: i16,
        /// Сколько чтений STATUS измерение ещё не готово; `None` — никогда.
        ready_after: Option<usize>,
        polls: usize,
    }

    impl Chip {
        fn new(temp: i16, ready_after: Option<usize>) -> Chip {
            Chip { id: 0xA9, ctrl: 0, temp, ready_after, polls: 0 }
        }

        fn get(&mut self, reg: u8) -> u8 {
            let [lo, hi] = self.temp.to_le_bytes();
            match reg {
                0x0F => self.id,
                0x20 => self.ctrl,
                0x27 if self.ctrl & 1 == 0 => 0,
                0x27 => {
                    self.polls += 1;
                    u8::from(self.ready_after.is_some_and(|n| self.polls > n))
                }
                0x2A => lo,
                0x2B => {
                    // Чтение старшего байта забирает измерение.
                    self.polls = 0;
                    hi
                }
                _ => 0,
            }
        }

        fn set(&mut self, reg: u8, value: u8) {
            assert_eq!(reg, 0x20, "запись в регистр {reg:#04x}: писать можно только в CTRL");
            self.ctrl = value;
        }
    }

    /// Шина I2C с одним TMP-9 по адресу `addr`.
    struct I2cSim {
        addr: u8,
        chip: Chip,
        /// Указатель регистра TMP-9: живёт между транзакциями.
        ptr: u8,
        log: Vec<String>,
    }

    impl I2cSim {
        fn new(chip: Chip) -> I2cSim {
            I2cSim { addr: 0x48, chip, ptr: 0, log: Vec::new() }
        }
    }

    impl i2c::ErrorType for I2cSim {
        type Error = i2c::ErrorKind;
    }

    impl I2c for I2cSim {
        fn transaction(
            &mut self,
            address: u8,
            ops: &mut [i2c::Operation<'_>],
        ) -> Result<(), Self::Error> {
            let mut line = format!("i2c {address:#04x}:");
            if address != self.addr {
                self.log.push(line + " NACK");
                return Err(i2c::ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address));
            }
            for op in ops.iter_mut() {
                match op {
                    // Первый байт записи — номер регистра, дальше данные.
                    i2c::Operation::Write(data) => {
                        line += &format!(" W{}", hex(data));
                        let (&reg, rest) = data.split_first().expect("пустая запись");
                        self.ptr = reg;
                        for &v in rest {
                            self.chip.set(self.ptr, v);
                            self.ptr += 1;
                        }
                    }
                    i2c::Operation::Read(buf) => {
                        line += &format!(" R{}", buf.len());
                        for b in buf.iter_mut() {
                            *b = self.chip.get(self.ptr);
                            self.ptr += 1;
                        }
                    }
                }
            }
            self.log.push(line);
            Ok(())
        }
    }

    /// Устройство SPI: TMP-9 со своим выбором кристалла.
    struct SpiSim {
        chip: Chip,
        /// Сколько транзакций пройдёт до сбоя шины; `None` — без сбоев.
        fail_after: Option<usize>,
        log: Vec<String>,
    }

    impl SpiSim {
        fn new(chip: Chip) -> SpiSim {
            SpiSim { chip, fail_after: None, log: Vec::new() }
        }
    }

    impl spi::ErrorType for SpiSim {
        type Error = spi::ErrorKind;
    }

    impl SpiDevice for SpiSim {
        fn transaction(&mut self, ops: &mut [spi::Operation<'_, u8>]) -> Result<(), Self::Error> {
            if self.fail_after == Some(self.log.len()) {
                self.log.push("spi: сбой".into());
                return Err(spi::ErrorKind::Other);
            }
            let mut line = String::from("spi:");
            // Первый байт транзакции — команда: бит 7 — чтение, биты 0–6 —
            // регистр. Указатель живёт до конца транзакции.
            let mut cmd: Option<(bool, u8)> = None;
            for op in ops.iter_mut() {
                match op {
                    spi::Operation::Write(data) => {
                        line += &format!(" W{}", hex(data));
                        let mut bytes = data.iter();
                        if cmd.is_none() {
                            let &c = bytes.next().expect("пустая запись");
                            cmd = Some((c & 0x80 != 0, c & 0x7f));
                        }
                        let (read, reg) = cmd.as_mut().unwrap();
                        for &v in bytes {
                            assert!(!*read, "данные для записи в транзакции чтения");
                            self.chip.set(*reg, v);
                            *reg += 1;
                        }
                    }
                    spi::Operation::Read(buf) => {
                        line += &format!(" R{}", buf.len());
                        let (read, reg) = cmd.as_mut().expect("чтение без команды");
                        assert!(*read, "чтение в транзакции записи: нет бита 7 в команде");
                        for b in buf.iter_mut() {
                            *b = self.chip.get(*reg);
                            *reg += 1;
                        }
                    }
                    other => panic!("ждали в транзакции только Write и Read, а не {other:?}"),
                }
            }
            self.log.push(line);
            Ok(())
        }
    }

    #[test]
    fn task_d_i2c() {
        let sim = I2cSim::new(Chip::new(2150, Some(2)));
        let mut tmp = Tmp9::new_i2c(sim, 0x48).unwrap();
        assert_eq!(tmp.temperature(), Ok(2150));
        let sim = tmp.release();
        assert_eq!(
            sim.log,
            [
                "i2c 0x48: W[0f] R1",
                "i2c 0x48: W[20 01]",
                "i2c 0x48: W[27] R1",
                "i2c 0x48: W[27] R1",
                "i2c 0x48: W[27] R1",
                "i2c 0x48: W[2a] R2",
            ]
        );
    }

    #[test]
    fn task_d_spi() {
        let sim = SpiSim::new(Chip::new(-405, Some(0)));
        let mut tmp = Tmp9::new_spi(sim).unwrap();
        assert_eq!(tmp.temperature(), Ok(-405));
        assert_eq!(tmp.temperature(), Ok(-405));
        let sim = tmp.release();
        assert_eq!(
            sim.log,
            [
                "spi: W[8f] R1",
                "spi: W[20 01]",
                "spi: W[a7] R1",
                "spi: W[aa] R2",
                "spi: W[a7] R1",
                "spi: W[aa] R2",
            ]
        );
    }

    #[test]
    fn task_d_wrong_id() {
        let mut chip = Chip::new(0, Some(0));
        chip.id = 0x33;
        // `&mut I2cSim` — тоже шина I2C: embedded-hal реализует I2c для
        // `&mut T`. Шина остаётся у теста, и журнал можно проверить.
        let mut sim = I2cSim::new(chip);
        assert_eq!(Tmp9::new_i2c(&mut sim, 0x48).err(), Some(Error::WrongId(0x33)));
        assert_eq!(sim.log, ["i2c 0x48: W[0f] R1"], "после чужого WHO_AM_I писать в CTRL нельзя");
    }

    #[test]
    fn task_d_nack() {
        let mut sim = I2cSim::new(Chip::new(0, Some(0)));
        assert_eq!(
            Tmp9::new_i2c(&mut sim, 0x49).err(),
            Some(Error::Bus(i2c::ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)))
        );
        assert_eq!(sim.log, ["i2c 0x49: NACK"]);
    }

    #[test]
    fn task_d_timeout() {
        let mut sim = SpiSim::new(Chip::new(0, None));
        let mut tmp = Tmp9::new_spi(&mut sim).unwrap();
        assert_eq!(tmp.temperature(), Err(Error::Timeout));
        let mut want = vec!["spi: W[8f] R1", "spi: W[20 01]"];
        want.extend(["spi: W[a7] R1"; 10]);
        assert_eq!(sim.log, want);
    }

    #[test]
    fn task_d_bus_error() {
        let mut sim = SpiSim::new(Chip::new(0, Some(0)));
        sim.fail_after = Some(2);
        let mut tmp = Tmp9::new_spi(&mut sim).unwrap();
        assert_eq!(tmp.temperature(), Err(Error::Bus(spi::ErrorKind::Other)));
        assert_eq!(sim.log, ["spi: W[8f] R1", "spi: W[20 01]", "spi: сбой"]);
    }
}
