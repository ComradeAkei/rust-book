// Общие тесты задач главы 24. Подключаются из exercises/ch24 и
// solutions/ch24 через `use <крейт> as sut; include!(...)`. Ошибки
// компиляции задач B–D проверяют снимки trybuild из tests-shared/ch24-ui
// (tests/ui.rs каждого крейта).

use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------- A

/// Без пробелов и строк-комментариев: сравнивается текст, а не вёрстка.
fn normalize(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .flat_map(str::chars)
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Строки от первой, что начинается с `start`, до первой после неё строки,
/// которая начинается с `end` (обе включительно).
fn lines_between(text: &str, start: &str, end: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let i = lines
        .iter()
        .position(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("нет {start:?}:\n{text}"));
    let j = i + lines[i..].iter().position(|l| l.trim_start().starts_with(end)).unwrap();
    lines[i..=j].join("\n")
}

/// Раскрытие прошивки firmware/ch24-expand: два impl Debug, переходник
/// `#[entry]` и начало функции пользователя.
fn expanded_firmware() -> [String; 4] {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../firmware");
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch24-a");
    let out = Command::new("cargo")
        .current_dir(&dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["rustc", "-p", "ch24-expand", "--profile", "check"])
        .arg("--target-dir")
        .arg(&target)
        .args(["--", "-Zunpretty=expanded"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    // Элемент верхнего уровня: от строки выше `head` (там атрибут) до
    // закрывающей скобки без отступа.
    let item = |head: &str| {
        let i = lines
            .iter()
            .position(|l| l.starts_with(head))
            .unwrap_or_else(|| panic!("нет {head:?}:\n{text}"));
        let j = i + lines[i..].iter().position(|l| *l == "}").unwrap();
        lines[i - 1..=j].join("\n")
    };
    [
        item("impl ::core::fmt::Debug for Point {"),
        item("impl ::core::fmt::Debug for Mode {"),
        item("#[export_name = \"main\"]"),
        lines_between(&text, "fn __cortex_m_rt_main(", "*BOOTS += 1;"),
    ]
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo и rustc")]
fn rustc_a_expansions_match_prediction() {
    let got = expanded_firmware();
    let want = sut::a_expand::predicted();
    let what = [
        "derive(Debug) для Point",
        "derive(Debug) для Mode",
        "переходник #[entry]",
        "функция пользователя",
    ];
    for ((g, w), what) in got.iter().zip(want).zip(what) {
        assert_eq!(normalize(g), normalize(w), "{what}: rustc раскрыл так:\n{g}");
    }
}

// ---------------------------------------------------------------- B

use sut::b_bitfield::{LCRH_8N1_FIFO, Lcrh, LcrhBits};

#[test]
fn b_layout() {
    assert_eq!(
        (LcrhBits::BRK_SHIFT, LcrhBits::FEN_SHIFT, LcrhBits::WLEN_SHIFT, LcrhBits::SPS_SHIFT),
        (0, 4, 5, 7)
    );
    assert_eq!((LcrhBits::BRK_MASK, LcrhBits::WLEN_MASK, LcrhBits::SPS_MASK), (0x01, 0x60, 0x80));
    assert_eq!(LcrhBits::WIDTH, 8);
    assert_eq!(size_of::<LcrhBits>(), 4);
}

#[test]
fn b_get_set() {
    let mut b = LcrhBits(0);
    b.set_wlen(3);
    b.set_fen(true);
    assert_eq!(b.0, 0x70);
    assert_eq!((b.wlen(), b.fen(), b.pen()), (3, true, false));
    b.set_fen(false);
    assert_eq!(b.0, 0x60);
    // Чужие биты слова не трогаются.
    assert_eq!(LcrhBits(0xFFFF_FF00).with_wlen(1).0, 0xFFFF_FF20);
    assert_eq!(LcrhBits(0xFF).with_wlen(0).0, 0x9F);
}

#[test]
fn b_const_and_debug() {
    // LCRH_8N1_FIFO собран при компиляции: with_* — const fn.
    assert_eq!(LCRH_8N1_FIFO.0, 0x70);
    assert_eq!(
        format!("{LCRH_8N1_FIFO:?}"),
        "LcrhBits { brk: false, pen: false, eps: false, stp2: false, fen: true, wlen: 3, sps: false }"
    );
}

#[test]
fn b_roundtrip() {
    let l = Lcrh { pen: true, eps: true, wlen: 2, ..Lcrh::default() };
    let b = LcrhBits::from(l);
    assert_eq!(b.0, 0b0100_0110);
    assert_eq!(Lcrh::from(b), l);
}

#[test]
#[should_panic(expected = "Lcrh.wlen: значение не помещается в 2 бит")]
fn b_value_too_wide_panics() {
    let _ = LcrhBits(0).with_wlen(4);
}

#[test]
fn b_full_word() {
    #[derive(sut::Bitfield)]
    struct Split {
        #[bits(16)]
        lo: u16,
        #[bits(12)]
        mid: u16,
        #[bits(4)]
        hi: u8,
    }
    assert_eq!(SplitBits::WIDTH, 32);
    assert_eq!(SplitBits(0).with_hi(0xF).0, 0xF000_0000);
    assert_eq!(SplitBits(0).with_lo(0xFFFF).0, 0x0000_FFFF);
    assert_eq!(SplitBits::MID_MASK, 0x0FFF_0000);
    assert_eq!((SplitBits::LO_SHIFT, SplitBits::MID_SHIFT, SplitBits::HI_SHIFT), (0, 16, 28));
    assert_eq!(SplitBits(0).with_mid(0xABC).0, 0x0ABC_0000);
    let s = Split::from(SplitBits(0x1234_5678));
    assert_eq!((s.lo, s.mid, s.hi), (0x5678, 0x234, 0x1));

    #[derive(sut::Bitfield)]
    struct Whole {
        #[bits(32)]
        all: u32,
    }
    assert_eq!(WholeBits::ALL_MASK, u32::MAX);
    assert_eq!(WholeBits(0).with_all(u32::MAX).0, u32::MAX);
    assert_eq!(Whole::from(WholeBits(7)).all, 7);
}

// ---------------------------------------------------------------- C

use sut::c_timed::{Clock, FakeClock, HostClock, Timing};

#[test]
fn c_counts_calls() {
    static WORK: Timing<FakeClock> = Timing::new();
    #[sut::timed(WORK)]
    fn work(ticks: u32) -> u32 {
        FakeClock::advance(ticks);
        ticks * 2
    }
    assert_eq!((work(5), work(7), work(1)), (10, 14, 2));
    assert_eq!((WORK.calls(), WORK.total(), WORK.max()), (3, 13, 7));
}

#[test]
fn c_every_exit_is_counted() {
    static PARSE: Timing<FakeClock> = Timing::new();
    #[sut::timed(PARSE)]
    fn parse(s: &str) -> Result<u8, std::num::ParseIntError> {
        FakeClock::advance(2);
        if s.is_empty() {
            return Ok(0);
        }
        let v: u8 = s.parse()?;
        FakeClock::advance(3);
        Ok(v)
    }
    assert_eq!(parse(""), Ok(0)); // return: 2
    assert!(parse("x").is_err()); // ?: 2
    assert_eq!(parse("42"), Ok(42)); // до конца: 5
    assert_eq!((PARSE.calls(), PARSE.total(), PARSE.max()), (3, 9, 5));
}

#[test]
fn c_panic_is_counted() {
    static BOOM: Timing<FakeClock> = Timing::new();
    #[sut::timed(BOOM)]
    fn boom() {
        FakeClock::advance(4);
        panic!("бум");
    }
    assert!(std::panic::catch_unwind(boom).is_err());
    assert_eq!((BOOM.calls(), BOOM.total()), (1, 4));
}

#[test]
fn c_arguments_keep_their_names() {
    static ARGS: Timing<FakeClock> = Timing::new();
    // Аргументы с именами, которые макрос мог бы взять себе.
    #[sut::timed(ARGS)]
    #[allow(clippy::used_underscore_binding, clippy::duplicate_underscore_argument)]
    fn sum(guard: u32, _guard: u32, __guard: u32, start: u32, timing: u32) -> u32 {
        guard + _guard + __guard + start + timing
    }
    assert_eq!(sum(1, 2, 3, 4, 5), 15);
    assert_eq!(ARGS.calls(), 1);
}

#[test]
fn c_methods_and_generics() {
    static GET: Timing<FakeClock> = Timing::new();
    static ID: Timing<FakeClock> = Timing::new();
    struct Sensor(u16);
    impl Sensor {
        #[sut::timed(GET)]
        fn get(&self, offset: u16) -> u16 {
            FakeClock::advance(1);
            self.0 + offset
        }
    }
    #[sut::timed(ID)]
    fn id<T: Copy>(x: T) -> T {
        x
    }
    assert_eq!(Sensor(40).get(2), 42);
    assert_eq!((id(1u8), id("a")), (1, "a"));
    assert_eq!((GET.calls(), GET.total(), ID.calls()), (1, 1, 2));
}

#[test]
fn c_host_clock_and_wrapping() {
    static SLEEPY: Timing<HostClock> = Timing::new();
    #[sut::timed(SLEEPY)]
    fn sleepy() {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    sleepy();
    assert!(SLEEPY.max() >= 2_000_000, "{} нс", SLEEPY.max());
    // Счётчик вверх переполняется: разность с переносом.
    assert_eq!(<FakeClock as Clock>::elapsed(u32::MAX - 1, 3), 5);
}

// ---------------------------------------------------------------- D

use sut::d_svd::{uart0, uart1};

#[test]
fn d_layout() {
    assert_eq!((uart0::BASE, uart1::BASE), (0x4000_C000, 0x4000_D000));
    assert_eq!(size_of::<uart0::RegisterBlock>(), 0x48);
    assert_eq!(size_of::<uart1::RegisterBlock>(), 0x48);
    let offsets = [
        std::mem::offset_of!(uart0::RegisterBlock, dr),
        std::mem::offset_of!(uart0::RegisterBlock, rsr),
        std::mem::offset_of!(uart0::RegisterBlock, fr),
        std::mem::offset_of!(uart0::RegisterBlock, ibrd),
        std::mem::offset_of!(uart0::RegisterBlock, lcrh),
        std::mem::offset_of!(uart0::RegisterBlock, ctl),
        std::mem::offset_of!(uart0::RegisterBlock, icr),
    ];
    assert_eq!(offsets, [0x00, 0x04, 0x18, 0x24, 0x2C, 0x30, 0x44]);
    assert_eq!((uart0::Fr::OFFSET, uart0::Icr::OFFSET), (0x18, 0x44));
}

#[test]
fn d_reset_values_and_fields() {
    assert_eq!((uart0::Ctl::RESET, uart0::Fr::RESET, uart0::Ifls::RESET), (0x300, 0x90, 0x12));
    assert_eq!(uart1::Ctl::RESET, 0x300);
    assert_eq!(uart0::Fr::TXFF, uart0::Field { offset: 5, width: 1 });
    assert_eq!(uart0::Lcrh::WLEN.mask(), 0x60);
    assert_eq!(uart0::Lcrh::WLEN.set(0xFF, 1), 0xBF);
    assert_eq!(uart0::Lcrh::WLEN.get(0x60), 3);
    assert_eq!(uart0::Ibrd::DIVINT.mask(), 0xFFFF);
    assert_eq!(uart0::Dr::DATA.mask(), 0xFF);
    // Поле во всё слово — край маски.
    assert_eq!(uart0::Field { offset: 0, width: 32 }.mask(), u32::MAX);
}

#[repr(C, align(4))]
struct Mem([u32; 0x48 / 4]);

#[test]
fn d_access_on_memory() {
    let mut mem = Mem([0; 0x48 / 4]);
    mem.0[0x18 / 4] = 0x90; // FR после сброса
    {
        // SAFETY: mem выровнен, размером с блок и живёт дольше uart.
        let uart = unsafe { uart0::RegisterBlock::from_ptr((&raw mut mem).cast()) };
        assert_eq!(uart0::Fr::TXFE.get(uart.fr.read()), 1);
        uart.ctl.write(0x301);
        uart.lcrh.modify(|v| uart0::Lcrh::WLEN.set(v, 3));
        uart.dr.write(u32::from(b'A'));
        uart.icr.write(0x7F0);
    }
    assert_eq!(mem.0[0x30 / 4], 0x301);
    assert_eq!(mem.0[0x2C / 4], 0x60);
    assert_eq!(mem.0[0], 0x41);
    assert_eq!(mem.0[0x44 / 4], 0x7F0);
}

/// Файл SVD — зависимость сборки: путь к нему есть в dep-info библиотеки,
/// и cargo пересоберёт её, когда файл изменится.
#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn rustc_d_svd_is_build_dependency() {
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch24-d");
    let out = Command::new("cargo")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["build", "--lib"])
        .arg("--target-dir")
        .arg(&target)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let lib = env!("CARGO_PKG_NAME").replace('-', "_");
    let deps = target.join("debug/deps");
    let dep_info = std::fs::read_dir(&deps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with(&format!("{lib}-")) && n.ends_with(".d")
        })
        .map(|p| std::fs::read_to_string(p).unwrap())
        .find(|d| d.contains(".rlib:"))
        .unwrap_or_else(|| panic!("нет dep-info {lib} в {}", deps.display()));
    assert!(dep_info.contains("code/ch24/svd/lm3s6965.svd"), "{dep_info}");
}
