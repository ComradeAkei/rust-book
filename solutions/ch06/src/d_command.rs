//! Задача 6.D: команды протокола в 8 байтах.
//!
//! Исходный `NaiveCmd` занимает 12 байт из-за варианта `Write { addr: u32,
//! value: u32 }`: 8 байт данных плюс тег, округлённые до выравнивания 4.
//! Решение опирается на знание предметной области: адрес регистра выровнен
//! на 4 и лежит в окне 256 КиБ, значит, хватает 16-битного индекса слова.
//!
//! Восемь байт должны быть гарантией, а не удачей. Без `repr` rustc сегодня
//! укладывает `Cmd` в 8 байт, но под `-Zrandomize-layout` тот же тип
//! занимает 12. `#[repr(u8)]` фиксирует раскладку по RFC 2195 (раздел 6.3).

use ch06::task_d::{NaiveCmd, WINDOW_BASE, WINDOW_LEN};

// ANCHOR: reg_idx
/// Номер 32-битного регистра в окне периферии.
///
/// `repr(transparent)` обещает раскладку ровно как у `u16`: от этого
/// зависит смещение `value` в варианте `Cmd::Write`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct RegIdx(u16);

impl RegIdx {
    pub const fn from_addr(addr: u32) -> Result<Self, ConvertError> {
        if addr % 4 != 0 {
            return Err(ConvertError::new(ConvertErrorKind::Misaligned, addr));
        }
        // wrapping_sub делает адреса ниже окна огромными, и одна проверка
        // отсекает выход за окно с обеих сторон.
        let offset = addr.wrapping_sub(WINDOW_BASE);
        if offset >= WINDOW_LEN {
            return Err(ConvertError::new(ConvertErrorKind::OutOfWindow, addr));
        }
        Ok(RegIdx((offset / 4) as u16))
    }

    pub const fn addr(self) -> u32 {
        WINDOW_BASE + self.0 as u32 * 4
    }
}
// ANCHOR_END: reg_idx

// ANCHOR: cmd
/// Тег в байте 0, `RegIdx` в байтах 2..4, `u32` в байтах 4..8.
/// Значения тега 6..=255 свободны: это ниша для `Option` и `Result`.
///
/// `repr(u8)` делает каждый вариант структурой `repr(C)` вида
/// `{ tag: u8, поля... }`, поэтому `Write` гарантированно занимает 8 байт:
/// тег, байт выравнивания, `u16`, `u32`. Без атрибута те же 8 байт —
/// лишь нынешнее решение rustc.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Cmd {
    Ping,
    Reset { delay_ms: u32 },
    SetLed { id: u8, on: bool },
    SetPwm { channel: u8, duty: u16 },
    Read { reg: RegIdx },
    Write { reg: RegIdx, value: u32 },
}
// ANCHOR_END: cmd

// ANCHOR: error
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ConvertErrorKind {
    OutOfWindow,
    Misaligned,
}

/// Ошибка с адресом, но с выравниванием 1 и размером 5 байт.
///
/// Вариант `Err` в `Result<Cmd, ConvertError>` должен лечь в те байты `Cmd`,
/// которые не пересекаются с нишей (байт 0). При выравнивании 4 ошибке
/// пришлось бы начинаться со смещения 0 или 4, и `Result` вырос бы до 12
/// байт. Массив `[u8; 4]` вместо `u32` позволяет положить её в байты 1..6.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConvertError {
    kind: ConvertErrorKind,
    addr: [u8; 4],
}

impl ConvertError {
    const fn new(kind: ConvertErrorKind, addr: u32) -> Self {
        Self { kind, addr: addr.to_ne_bytes() }
    }
    pub const fn kind(&self) -> ConvertErrorKind {
        self.kind
    }
    pub const fn addr(&self) -> u32 {
        u32::from_ne_bytes(self.addr)
    }
}
// ANCHOR_END: error

// ANCHOR: convert
impl TryFrom<NaiveCmd> for Cmd {
    type Error = ConvertError;

    fn try_from(c: NaiveCmd) -> Result<Self, ConvertError> {
        Ok(match c {
            NaiveCmd::Ping => Cmd::Ping,
            NaiveCmd::Reset { delay_ms } => Cmd::Reset { delay_ms },
            NaiveCmd::SetLed { id, on } => Cmd::SetLed { id, on },
            NaiveCmd::SetPwm { channel, duty } => Cmd::SetPwm { channel, duty },
            NaiveCmd::Read { addr } => Cmd::Read { reg: RegIdx::from_addr(addr)? },
            NaiveCmd::Write { addr, value } => Cmd::Write { reg: RegIdx::from_addr(addr)?, value },
        })
    }
}

impl From<Cmd> for NaiveCmd {
    fn from(c: Cmd) -> Self {
        match c {
            Cmd::Ping => NaiveCmd::Ping,
            Cmd::Reset { delay_ms } => NaiveCmd::Reset { delay_ms },
            Cmd::SetLed { id, on } => NaiveCmd::SetLed { id, on },
            Cmd::SetPwm { channel, duty } => NaiveCmd::SetPwm { channel, duty },
            Cmd::Read { reg } => NaiveCmd::Read { addr: reg.addr() },
            Cmd::Write { reg, value } => NaiveCmd::Write { addr: reg.addr(), value },
        }
    }
}
// ANCHOR_END: convert

// Размер `Cmd` гарантирует repr(u8). Ниша в теге для `Option` и `Result`
// не документирована, но от порядка полей не зависит: обе проверки
// проходят и под -Zrandomize-layout. Если rustc когда-нибудь откажется от
// такой оптимизации, сборка остановится здесь, а не в очереди команд.
const _: () = assert!(core::mem::size_of::<Cmd>() == 8);
const _: () = assert!(core::mem::size_of::<Option<Cmd>>() == 8);
const _: () = assert!(core::mem::size_of::<Result<Cmd, ConvertError>>() == 8);
