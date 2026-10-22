//! Задача 2.B: подмножество 16-битных инструкций Thumb (ARMv6-M) и их
//! запись в синтаксисе `llvm-objdump`.
//!
//! Декодер пишет студент (`exercises/ch02/src/b_thumb.rs`), а тип
//! инструкции и её печать даны здесь: тесты сравнивают напечатанную
//! инструкцию со строкой дизассемблера.

use core::fmt;

/// Условие перехода `b<c>`: поле `cond` в битах 11–8. Значения 14 и 15 —
/// не переходы, а `udf` и `svc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    Eq = 0,
    Ne = 1,
    Hs = 2,
    Lo = 3,
    Mi = 4,
    Pl = 5,
    Vs = 6,
    Vc = 7,
    Hi = 8,
    Ls = 9,
    Ge = 10,
    Lt = 11,
    Gt = 12,
    Le = 13,
}

impl Cond {
    /// Условие по полю `cond`, `None` для 14 и 15.
    pub fn from_bits(bits: u16) -> Option<Cond> {
        const ALL: [Cond; 14] = [
            Cond::Eq,
            Cond::Ne,
            Cond::Hs,
            Cond::Lo,
            Cond::Mi,
            Cond::Pl,
            Cond::Vs,
            Cond::Vc,
            Cond::Hi,
            Cond::Ls,
            Cond::Ge,
            Cond::Lt,
            Cond::Gt,
            Cond::Le,
        ];
        ALL.get(usize::from(bits)).copied()
    }

    fn suffix(self) -> &'static str {
        match self {
            Cond::Eq => "eq",
            Cond::Ne => "ne",
            Cond::Hs => "hs",
            Cond::Lo => "lo",
            Cond::Mi => "mi",
            Cond::Pl => "pl",
            Cond::Vs => "vs",
            Cond::Vc => "vc",
            Cond::Hi => "hi",
            Cond::Ls => "ls",
            Cond::Ge => "ge",
            Cond::Lt => "lt",
            Cond::Gt => "gt",
            Cond::Le => "le",
        }
    }
}

// ANCHOR: insn
/// Инструкции подмножества. Поля регистров — номера 0–15, поля `imm` —
/// уже масштабированные значения: смещение в байтах, а не в словах.
/// Смещения переходов — в байтах относительно адреса инструкции плюс 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insn {
    /// `movs rd, #imm` — `00100 ddd iiiiiiii`.
    MovsImm { rd: u8, imm: u8 },
    /// `movs rd, rm` — `00000 00000 mmm ddd` (сдвиг LSL на 0).
    MovsReg { rd: u8, rm: u8 },
    /// `mov rd, rm` — `010001 10 D mmmm ddd`, rd = D:ddd, любые r0–r15.
    Mov { rd: u8, rm: u8 },
    /// `adds rd, rn, rm` — `0001100 mmm nnn ddd`.
    AddsReg { rd: u8, rn: u8, rm: u8 },
    /// `adds rd, rn, #imm` — `0001110 iii nnn ddd`.
    AddsImm3 { rd: u8, rn: u8, imm: u8 },
    /// `adds rdn, #imm` — `00110 ddd iiiiiiii`.
    AddsImm8 { rdn: u8, imm: u8 },
    /// `add rdn, rm` — `010001 00 D mmmm ddd`, rdn = D:ddd, любые r0–r15.
    Add { rdn: u8, rm: u8 },
    /// `add rd, sp, #imm` — `10101 ddd iiiiiiii`, imm = imm8 · 4.
    AddRdSp { rd: u8, imm: u16 },
    /// `add sp, #imm` — `1011 0000 0 iiiiiii`, imm = imm7 · 4.
    AddSp { imm: u16 },
    /// `subs rd, rn, rm` — `0001101 mmm nnn ddd`.
    SubsReg { rd: u8, rn: u8, rm: u8 },
    /// `subs rd, rn, #imm` — `0001111 iii nnn ddd`.
    SubsImm3 { rd: u8, rn: u8, imm: u8 },
    /// `subs rdn, #imm` — `00111 ddd iiiiiiii`.
    SubsImm8 { rdn: u8, imm: u8 },
    /// `sub sp, #imm` — `1011 0000 1 iiiiiii`, imm = imm7 · 4.
    SubSp { imm: u16 },
    /// `b<c> метка` — `1101 cccc iiiiiiii`, смещение imm8 · 2 со знаком.
    BCond { cond: Cond, offset: i32 },
    /// `b метка` — `11100 iiiiiiiiiii`, смещение imm11 · 2 со знаком.
    B { offset: i32 },
    /// `ldr rt, [rn, #imm]` — `01101 iiiii nnn ttt`, imm = imm5 · 4.
    LdrImm { rt: u8, rn: u8, imm: u8 },
    /// `ldr rt, [sp, #imm]` — `10011 ttt iiiiiiii`, imm = imm8 · 4.
    LdrSp { rt: u8, imm: u16 },
    /// `ldr rt, [pc, #imm]` — `01001 ttt iiiiiiii`, imm = imm8 · 4.
    LdrLit { rt: u8, imm: u16 },
    /// `ldr rt, [rn, rm]` — `0101100 mmm nnn ttt`.
    LdrReg { rt: u8, rn: u8, rm: u8 },
}
// ANCHOR_END: insn

/// Имя регистра так, как его пишет `llvm-objdump`.
struct Reg(u8);

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            13 => f.write_str("sp"),
            14 => f.write_str("lr"),
            15 => f.write_str("pc"),
            n => write!(f, "r{n}"),
        }
    }
}

/// Инструкция по адресу `addr`: адрес нужен, чтобы напечатать цель
/// перехода так же, как `llvm-objdump`.
pub struct Disasm {
    pub insn: Insn,
    pub addr: u32,
}

impl fmt::Display for Disasm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let target = |offset: i32| self.addr.wrapping_add(4).wrapping_add_signed(offset);
        match self.insn {
            Insn::MovsImm { rd, imm } => write!(f, "movs\t{}, #{imm:#x}", Reg(rd)),
            Insn::MovsReg { rd, rm } => write!(f, "movs\t{}, {}", Reg(rd), Reg(rm)),
            Insn::Mov { rd, rm } => write!(f, "mov\t{}, {}", Reg(rd), Reg(rm)),
            Insn::AddsReg { rd, rn, rm } => {
                write!(f, "adds\t{}, {}, {}", Reg(rd), Reg(rn), Reg(rm))
            }
            Insn::AddsImm3 { rd, rn, imm } => {
                write!(f, "adds\t{}, {}, #{imm:#x}", Reg(rd), Reg(rn))
            }
            Insn::AddsImm8 { rdn, imm } => write!(f, "adds\t{}, #{imm:#x}", Reg(rdn)),
            // Если второй операнд — sp, дизассемблер пишет форму
            // «sp плюс регистр» с тремя операндами.
            Insn::Add { rdn, rm: 13 } => write!(f, "add\t{}, sp, {}", Reg(rdn), Reg(rdn)),
            Insn::Add { rdn, rm } => write!(f, "add\t{}, {}", Reg(rdn), Reg(rm)),
            Insn::AddRdSp { rd, imm } => write!(f, "add\t{}, sp, #{imm:#x}", Reg(rd)),
            Insn::AddSp { imm } => write!(f, "add\tsp, #{imm:#x}"),
            Insn::SubsReg { rd, rn, rm } => {
                write!(f, "subs\t{}, {}, {}", Reg(rd), Reg(rn), Reg(rm))
            }
            Insn::SubsImm3 { rd, rn, imm } => {
                write!(f, "subs\t{}, {}, #{imm:#x}", Reg(rd), Reg(rn))
            }
            Insn::SubsImm8 { rdn, imm } => write!(f, "subs\t{}, #{imm:#x}", Reg(rdn)),
            Insn::SubSp { imm } => write!(f, "sub\tsp, #{imm:#x}"),
            Insn::BCond { cond, offset } => {
                write!(f, "b{}\t{:#x}", cond.suffix(), target(offset))
            }
            Insn::B { offset } => write!(f, "b\t{:#x}", target(offset)),
            Insn::LdrImm { rt, rn, imm: 0 } => write!(f, "ldr\t{}, [{}]", Reg(rt), Reg(rn)),
            Insn::LdrImm { rt, rn, imm } => {
                write!(f, "ldr\t{}, [{}, #{imm:#x}]", Reg(rt), Reg(rn))
            }
            Insn::LdrSp { rt, imm: 0 } => write!(f, "ldr\t{}, [sp]", Reg(rt)),
            Insn::LdrSp { rt, imm } => write!(f, "ldr\t{}, [sp, #{imm:#x}]", Reg(rt)),
            // У загрузки относительно pc нулевое смещение печатается явно.
            Insn::LdrLit { rt, imm } => write!(f, "ldr\t{}, [pc, #{imm:#x}]", Reg(rt)),
            Insn::LdrReg { rt, rn, rm } => {
                write!(f, "ldr\t{}, [{}, {}]", Reg(rt), Reg(rn), Reg(rm))
            }
        }
    }
}
