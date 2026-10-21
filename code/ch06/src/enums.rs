//! Раздел 6.3: enum как размеченное объединение.

// ANCHOR: c_style
/// Перечисление без данных. Компилятор выбирает самый узкий тип тега.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Idle,
    Measure,
    Calibrate,
}

/// Явные значения тега и фиксированный тип `u8`: так описывают коды протокола.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Opcode {
    Ping = 0x01,
    Reset = 0x02,
    Read = 0x10,
    Write = 0x11,
}
// ANCHOR_END: c_style

// ANCHOR: try_from
/// Обратное преобразование из байта нельзя сделать через `as`: не каждый байт
/// является допустимым тегом. Явная проверка превращает это в `Result`.
impl TryFrom<u8> for Opcode {
    type Error = u8;

    fn try_from(byte: u8) -> Result<Self, u8> {
        match byte {
            0x01 => Ok(Opcode::Ping),
            0x02 => Ok(Opcode::Reset),
            0x10 => Ok(Opcode::Read),
            0x11 => Ok(Opcode::Write),
            other => Err(other),
        }
    }
}
// ANCHOR_END: try_from

// ANCHOR: tagged
/// Сообщение от датчика: варианты несут разные данные.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Message {
    Heartbeat,
    Temperature { sensor: u8, centi_celsius: i16 },
    Pressure(u32),
    Fault { code: u16 },
}
// ANCHOR_END: tagged

// ANCHOR: describe
pub fn describe(m: &Message) -> u32 {
    match *m {
        Message::Heartbeat => 0,
        Message::Temperature { centi_celsius, .. } => centi_celsius as u32,
        Message::Pressure(pa) => pa,
        Message::Fault { code } => 0x8000_0000 | code as u32,
    }
}
// ANCHOR_END: describe

// ANCHOR: repr_c_enum
/// `repr(C)` для enum с данными даёт раскладку «struct { tag; union {...} }»,
/// которую C-код может прочитать напрямую. Размер тега берётся из C ABI
/// цели: 4 байта на x86_64 Linux, 1 байт на thumb*-none-eabi, где enum в C
/// сжимается до наименьшего подходящего типа.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum MessageC {
    Heartbeat,
    Temperature { sensor: u8, centi_celsius: i16 },
    Pressure(u32),
    Fault { code: u16 },
}

/// `repr(u8)` фиксирует тип тега, но оставляет ту же схему «тег + объединение».
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum MessageU8 {
    Heartbeat,
    Temperature { sensor: u8, centi_celsius: i16 },
    Pressure(u32),
    Fault { code: u16 },
}
// ANCHOR_END: repr_c_enum

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn fieldless_enum_is_one_byte() {
        assert_eq!(size_of::<Mode>(), 1);
        assert_eq!(size_of::<Opcode>(), 1);
        assert_eq!(Opcode::Write as u8, 0x11);
    }

    #[test]
    fn opcode_round_trip() {
        for op in [Opcode::Ping, Opcode::Reset, Opcode::Read, Opcode::Write] {
            assert_eq!(Opcode::try_from(op as u8), Ok(op));
        }
        assert_eq!(Opcode::try_from(0xFF), Err(0xFF));
    }

    #[test]
    fn tagged_union_size() {
        // Самый крупный вариант: u32 с выравниванием 4, плюс тег.
        assert_eq!(size_of::<Message>(), 8);
        assert_eq!(align_of::<Message>(), 4);
    }

    #[test]
    fn repr_c_enum_uses_int_tag() {
        // Тег repr(C) на x86_64 занимает 4 байта, на Cortex-M 1 байт, но
        // объединение вариантов выровнено на 4, поэтому размер везде 8.
        assert_eq!(size_of::<MessageC>(), 8);
        assert_eq!(size_of::<MessageU8>(), 8);
        let m = MessageU8::Pressure(7);
        // SAFETY: у repr(u8) enum первый байт по определению является тегом.
        let tag = unsafe { *(&m as *const MessageU8 as *const u8) };
        assert_eq!(tag, 2);
    }

    #[test]
    fn describe_works() {
        let t = Message::Temperature { sensor: 1, centi_celsius: 2150 };
        assert_eq!(describe(&t), 2150);
        assert_eq!(describe(&Message::Fault { code: 3 }), 0x8000_0003);
    }
}
