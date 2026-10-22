//! Задача 2.B: декодер подмножества 16-битных инструкций Thumb.

use ch02::thumb::{Cond, Insn};

// ANCHOR: decode
/// Разбирает 16-битное слово инструкции. `None` — инструкция вне
/// подмножества (или не инструкция вовсе, как `udf`).
pub fn decode(hw: u16) -> Option<Insn> {
    // Трёхбитное поле регистра, начиная с бита `at`.
    let reg = |at: u16| ((hw >> at) & 0b111) as u8;
    let imm8 = (hw & 0xFF) as u8;
    let imm8x4 = u16::from(imm8) * 4;
    // Регистр r0–r15 из бита D (7) и трёх младших битов.
    let high_rd = ((hw >> 4) & 0b1000) as u8 | reg(0);
    let high_rm = ((hw >> 3) & 0b1111) as u8;

    // Старшие пять битов определяют группу кодировок, дальше уточняют
    // следующие биты.
    let insn = match hw >> 11 {
        // LSL с нулевым сдвигом дизассемблер пишет как `movs`.
        0b00000 if hw & 0x07C0 == 0 => Insn::MovsReg { rd: reg(0), rm: reg(3) },
        0b00011 => match (hw >> 9) & 0b11 {
            0b00 => Insn::AddsReg { rd: reg(0), rn: reg(3), rm: reg(6) },
            0b01 => Insn::SubsReg { rd: reg(0), rn: reg(3), rm: reg(6) },
            0b10 => Insn::AddsImm3 { rd: reg(0), rn: reg(3), imm: reg(6) },
            _ => Insn::SubsImm3 { rd: reg(0), rn: reg(3), imm: reg(6) },
        },
        0b00100 => Insn::MovsImm { rd: reg(8), imm: imm8 },
        0b00110 => Insn::AddsImm8 { rdn: reg(8), imm: imm8 },
        0b00111 => Insn::SubsImm8 { rdn: reg(8), imm: imm8 },
        0b01000 => match (hw >> 8) & 0b111 {
            0b100 => Insn::Add { rdn: high_rd, rm: high_rm },
            0b110 => Insn::Mov { rd: high_rd, rm: high_rm },
            _ => return None,
        },
        0b01001 => Insn::LdrLit { rt: reg(8), imm: imm8x4 },
        0b01011 if (hw >> 9) & 0b11 == 0b00 => Insn::LdrReg { rt: reg(0), rn: reg(3), rm: reg(6) },
        0b01101 => Insn::LdrImm { rt: reg(0), rn: reg(3), imm: ((hw >> 6) & 0x1F) as u8 * 4 },
        0b10011 => Insn::LdrSp { rt: reg(8), imm: imm8x4 },
        0b10101 => Insn::AddRdSp { rd: reg(8), imm: imm8x4 },
        0b10110 if (hw >> 8) & 0b111 == 0 => {
            let imm = (hw & 0x7F) * 4;
            if hw & 0x80 == 0 { Insn::AddSp { imm } } else { Insn::SubSp { imm } }
        }
        0b11010 | 0b11011 => Insn::BCond {
            cond: Cond::from_bits((hw >> 8) & 0xF)?,
            offset: i32::from(imm8 as i8) * 2,
        },
        // Сдвиг влево и арифметический вправо на 5 распространяют знак
        // 11-битного поля.
        0b11100 => Insn::B { offset: i32::from((hw << 5) as i16 >> 5) * 2 },
        _ => return None,
    };
    Some(insn)
}
// ANCHOR_END: decode
