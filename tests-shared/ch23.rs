// Общие тесты задач главы 23. Подключаются из exercises/ch23 и
// solutions/ch23 через `use <крейт> as sut; include!(...)`.

use std::any::TypeId;
use sut::c_periph::{RO, RW, Reg, WO};

// ---------------------------------------------------------------- A

/// Те же шесть вызовов, подключённые модулем: поведение раскрытия.
mod a_calls {
    // swap! здесь нарочно ручной: его раскрытие — предмет задачи.
    #![allow(clippy::manual_swap)]
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../code/ch23/fragments/a_calls.rs"));
}

fn fragment_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../code/ch23/fragments")
}

/// Без пробелов и пустых операторов: сравнивается текст, а не вёрстка.
fn normalize(s: &str) -> String {
    let mut t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    while t.contains(";;") {
        t = t.replace(";;", ";");
    }
    t.replace("{;", "{")
}

/// Содержимое фигурных скобок, открытых первой `{` после `from`.
fn braced(text: &str, from: usize, keep_braces: bool) -> String {
    let open = from + text[from..].find('{').unwrap();
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = open + i;
                    return if keep_braces {
                        text[from..=end].to_string()
                    } else {
                        text[open + 1..end].to_string()
                    };
                }
            }
            _ => {}
        }
    }
    panic!("нет закрывающей скобки");
}

/// Раскрытие фрагмента закреплённым rustc: тела `call1`…`call5` и блок
/// `impl Point`.
fn expanded() -> Vec<String> {
    let out = std::process::Command::new("rustc")
        .current_dir(fragment_dir())
        .env_remove("RUSTUP_TOOLCHAIN")
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["--edition", "2024", "--crate-type", "lib", "-Zunpretty=expanded"])
        .arg("a_calls.rs")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).unwrap();
    let mut v: Vec<String> = (1..=5)
        .map(|n| {
            let at = text.find(&format!("pub fn call{n}(")).unwrap();
            braced(&text, at, false)
        })
        .collect();
    // Блок с начала строки: в комментарии файла слова `impl Point` тоже есть.
    v.push(braced(&text, text.find("\nimpl Point").unwrap() + 1, true));
    v
}

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn rustc_a_expansions_match_prediction() {
    let got = expanded();
    let want = sut::a_expand::predicted();
    for (n, (g, w)) in got.iter().zip(want).enumerate() {
        assert_eq!(normalize(g), normalize(w), "вызов {}: rustc раскрыл так:\n{g}", n + 1);
    }
}

#[test]
fn a_behaviour() {
    let mut calls = 0;
    let mut next = || {
        calls += 1;
        calls * 10
    };
    assert_eq!(a_calls::call1(&mut next), 20);
    assert_eq!(calls, 2, "next() вызвана дважды");
    assert_eq!(a_calls::call2(), (9, 5));
    assert_eq!(a_calls::call3(1, 2), (2, 1), "swap работает и с переменной tmp");
    assert_eq!(a_calls::call4(), 3);
    assert_eq!(a_calls::call5(), ("ноль", "не ноль"));
    let p = a_calls::Point { x: 3, y: -4 };
    assert_eq!((p.x(), p.y()), (3, -4));
}

// ---------------------------------------------------------------- B

sut::flags! {
    /// Регистр управления PL011 (UARTCR), используемые биты.
    pub struct Cr: u32 {
        const UARTEN = 1 << 0;
        /// Петля: передатчик замкнут на приёмник.
        const LBE = 1 << 7;
        const TXE = 1 << 8;
        const RXE = 1 << 9;
    }
}

sut::flags! {
    struct Status: u8 {
        const READY = 0x01;
        const BUSY = 0x02;
        const ERROR = 0x80;
    }
}

/// Вызывающий с враждебными именами: свой `Option`, свой `Result`, свой
/// модуль `core`. Макрос, который писал бы пути без `::core`, здесь бы
/// сломался.
#[allow(dead_code)]
mod hostile {
    pub struct Option;
    pub struct Result;
    pub mod core {}
    pub mod fmt {}
    super::sut::flags! {
        pub struct Mask: u16 {
            const LOW = 0x00FF;
            const HIGH = 0xFF00;
        }
    }
}

#[test]
fn b_constants_and_bits() {
    assert_eq!(Cr::UARTEN.bits(), 1);
    assert_eq!(Cr::RXE.bits(), 0x200);
    assert_eq!(Cr::empty().bits(), 0);
    assert_eq!(Cr::all().bits(), 0x381);
    assert!(Cr::empty().is_empty());
    assert!(!Cr::TXE.is_empty());
    assert_eq!(Status::all().bits(), 0x83);
    assert_eq!(hostile::Mask::all().bits(), 0xFFFF);
}

#[test]
fn b_operators() {
    let on = Cr::UARTEN | Cr::TXE | Cr::RXE;
    assert_eq!(on.bits(), 0x301);
    assert_eq!((on & Cr::TXE), Cr::TXE);
    assert_eq!((on & Cr::LBE), Cr::empty());
    assert_eq!(!Cr::TXE, Cr::UARTEN | Cr::LBE | Cr::RXE, "! — в пределах флагов");
    assert_eq!((!Cr::empty()), Cr::all());
    let mut v = Cr::UARTEN;
    v |= Cr::LBE;
    assert_eq!(v.bits(), 0x81);
    v &= Cr::LBE;
    assert_eq!(v, Cr::LBE);
    assert_eq!(on.union(Cr::LBE), Cr::all());
    assert_eq!(on.intersection(Cr::RXE | Cr::LBE), Cr::RXE);
    assert_eq!(on.difference(Cr::TXE), Cr::UARTEN | Cr::RXE);
    assert_eq!(Cr::all().complement(), Cr::empty());
    let mut w = Cr::empty();
    w.insert(Cr::TXE | Cr::RXE);
    w.remove(Cr::TXE);
    assert_eq!(w, Cr::RXE);
}

#[test]
fn b_queries() {
    let on = Cr::UARTEN | Cr::TXE;
    assert!(on.contains(Cr::TXE));
    assert!(on.contains(Cr::UARTEN | Cr::TXE));
    assert!(!on.contains(Cr::TXE | Cr::RXE));
    assert!(on.contains(Cr::empty()));
    assert!(on.intersects(Cr::TXE | Cr::RXE));
    assert!(!on.intersects(Cr::RXE | Cr::LBE));
}

#[test]
fn b_from_bits() {
    assert_eq!(Cr::from_bits(0x301), Some(Cr::UARTEN | Cr::TXE | Cr::RXE));
    assert_eq!(Cr::from_bits(0x305), None, "бит 2 не объявлен");
    assert_eq!(Cr::from_bits_truncate(0x305), Cr::UARTEN | Cr::TXE | Cr::RXE);
    assert_eq!(Cr::from_bits_retain(0x305).bits(), 0x305);
    assert_eq!(Status::from_bits(0x04), None);
}

#[test]
fn b_debug() {
    assert_eq!(format!("{:?}", Cr::UARTEN | Cr::TXE | Cr::RXE), "Cr(UARTEN | TXE | RXE)");
    assert_eq!(format!("{:?}", Cr::empty()), "Cr(empty)");
    assert_eq!(format!("{:?}", Cr::LBE), "Cr(LBE)");
    assert_eq!(format!("{:?}", Cr::from_bits_retain(0x104)), "Cr(TXE | 0x4)");
    assert_eq!(format!("{:?}", Cr::from_bits_retain(0x4)), "Cr(0x4)");
    assert_eq!(format!("{:?}", Status::READY | Status::ERROR), "Status(READY | ERROR)");
    assert_eq!(format!("{:?}", hostile::Mask::all()), "Mask(LOW | HIGH)");
}

#[test]
fn b_is_a_plain_integer() {
    assert_eq!(size_of::<Cr>(), 4);
    assert_eq!(size_of::<Status>(), 1);
    let set: std::collections::HashSet<Cr> = [Cr::TXE, Cr::TXE, Cr::RXE].into_iter().collect();
    assert_eq!(set.len(), 2);
}

// ---------------------------------------------------------------- C

sut::peripheral! {
    /// UART0 машины lm3s6965evb (PL011).
    pub UART0 @ 0x4000_C000 {
        DR: u32 @ 0x000,
        /// Флаги состояния — только чтение.
        FR: u32 @ 0x018 ro,
        IBRD: u32 @ 0x024,
        CR: u32 @ 0x030 rw,
        /// Сброс прерываний — только запись.
        ICR: u32 @ 0x044 wo,
    }
}

sut::peripheral! {
    TIMER @ 0x4003_0000 {
        CTL: u8 @ 0x0,
        PRESCALE: u16 @ 0x2 ro,
    }
}

/// Блок регистров на обычной памяти: 0x48 байт, выровнен под u32.
#[repr(C, align(4))]
struct Block([u32; 18]);

#[test]
fn c_base_and_types() {
    assert_eq!(UART0::BASE, 0x4000_C000);
    assert_eq!(TIMER::BASE, 0x4003_0000);
    let mut block = Block([0; 18]);
    // SAFETY: блок покрывает все регистры UART0 и выровнен под u32.
    let uart = unsafe { UART0::at(block.0.as_mut_ptr().cast()) };
    // Права доступа — в типе. Сравнение через TypeId, а не аннотацией
    // `let _: Reg<u32, RO>`: заготовка с одним правом иначе не собралась бы.
    assert_eq!(access(&uart.DR()), TypeId::of::<RW>());
    assert_eq!(access(&uart.FR()), TypeId::of::<RO>(), "FR — ro");
    assert_eq!(access(&uart.CR()), TypeId::of::<RW>());
    assert_eq!(access(&uart.ICR()), TypeId::of::<WO>(), "ICR — wo");
}

fn access<T, A: 'static>(_: &Reg<T, A>) -> TypeId {
    TypeId::of::<A>()
}

#[test]
fn c_read_write_at_offsets() {
    let mut block = Block([0; 18]);
    let base = block.0.as_mut_ptr();
    // SAFETY: как выше.
    let uart = unsafe { UART0::at(base.cast()) };
    uart.DR().write(b'A' as u32);
    uart.CR().write(0x301);
    uart.ICR().write(0x7FF);
    uart.IBRD().modify(|v| v + 26);
    // SAFETY: тот же блок; uart больше не обращается к памяти.
    let raw = unsafe { std::slice::from_raw_parts(base, 18) };
    assert_eq!(raw[0], 0x41, "DR по смещению 0x000");
    assert_eq!(raw[0x24 / 4], 26, "IBRD по смещению 0x024");
    assert_eq!(raw[0x30 / 4], 0x301, "CR по смещению 0x030");
    assert_eq!(raw[0x44 / 4], 0x7FF, "ICR по смещению 0x044");
    // FR только для чтения: значение кладёт «устройство».
    // SAFETY: запись в память блока мимо uart, как это сделало бы железо.
    unsafe { base.add(0x18 / 4).write_volatile(0x90) };
    assert_eq!(uart.FR().read(), 0x90);
    assert_eq!(uart.CR().read(), 0x301);
}

#[test]
fn c_narrow_registers() {
    let mut block = Block([0; 18]);
    let base = block.0.as_mut_ptr().cast::<u8>();
    // SAFETY: блок покрывает 4 байта TIMER и выровнен.
    let timer = unsafe { TIMER::at(base) };
    timer.CTL().write(0xA5);
    // SAFETY: запись «устройства» в PRESCALE (смещение 2, u16).
    unsafe { base.add(2).cast::<u16>().write_volatile(0x1234) };
    assert_eq!(timer.PRESCALE().read(), 0x1234);
    assert_eq!(timer.CTL().read(), 0xA5);
    // SAFETY: чтение того же блока байтами.
    assert_eq!(unsafe { base.read() }, 0xA5, "CTL — один байт по смещению 0");
}

// ---------------------------------------------------------------- D

sut::fsm! {
    /// Канал связи станции с хостом.
    pub enum Link { Idle, Dialing, Online, Failed }
    /// События канала.
    pub enum LinkEvent { Dial, Ack, Timeout, Hangup, Reset }
    Idle --Dial--> Dialing,
    _ --Reset--> Idle,
    Dialing --Ack--> Online,
    Dialing | Online --Timeout--> Failed,
    Online --Hangup--> Idle,
}

#[test]
fn d_transitions() {
    use Link::*;
    use LinkEvent::*;
    assert_eq!(Idle.next(Dial), Some(Dialing));
    assert_eq!(Dialing.next(Ack), Some(Online));
    assert_eq!(Dialing.next(Timeout), Some(Failed));
    assert_eq!(Online.next(Timeout), Some(Failed));
    assert_eq!(Online.next(Hangup), Some(Idle));
    assert_eq!(Idle.next(Ack), None);
    assert_eq!(Failed.next(Dial), None);
    // `_` — из любого состояния, хотя написан вторым.
    for &s in Link::STATES {
        assert_eq!(s.next(Reset), Some(Idle), "{s:?}");
    }
}

#[test]
fn d_table() {
    assert_eq!(Link::STATES, &[Link::Idle, Link::Dialing, Link::Online, Link::Failed]);
    let t = Link::TRANSITIONS;
    assert_eq!(t.len(), 6, "{t:?}");
    // Конкретные — в порядке записи, `_` — в конце.
    assert_eq!(t[0], (Some(Link::Idle), LinkEvent::Dial, Link::Dialing));
    assert_eq!(t[2], (Some(Link::Dialing), LinkEvent::Timeout, Link::Failed));
    assert_eq!(t[3], (Some(Link::Online), LinkEvent::Timeout, Link::Failed));
    assert_eq!(t[5], (None, LinkEvent::Reset, Link::Idle));
    // Таблица и `next` согласны.
    for &(from, ev, to) in t {
        if let Some(from) = from {
            assert_eq!(from.next(ev), Some(to));
        }
    }
}

#[test]
fn d_dot() {
    assert_eq!(
        Link::DOT,
        "digraph Link {\n\
         \x20 Idle -> Dialing [label=\"Dial\"];\n\
         \x20 Idle -> Idle [label=\"Reset\"];\n\
         \x20 Dialing -> Idle [label=\"Reset\"];\n\
         \x20 Online -> Idle [label=\"Reset\"];\n\
         \x20 Failed -> Idle [label=\"Reset\"];\n\
         \x20 Dialing -> Online [label=\"Ack\"];\n\
         \x20 Dialing -> Failed [label=\"Timeout\"];\n\
         \x20 Online -> Failed [label=\"Timeout\"];\n\
         \x20 Online -> Idle [label=\"Hangup\"];\n\
         }\n"
    );
}
