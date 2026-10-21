//! Исходные данные задачи 6.D «Команды протокола в 8 байтах».
//!
//! `NaiveCmd` — команда в том виде, в каком её прислал бы наивный парсер.
//! Она занимает 12 байт. Задача: спроектировать `Cmd`, который несёт ту же
//! информацию для допустимых команд и занимает 8 байт, причём `Option<Cmd>`
//! и `Result<Cmd, ConvertError>` тоже остаются в 8 байтах.

// ANCHOR: naive
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NaiveCmd {
    Ping,
    Reset { delay_ms: u32 },
    SetLed { id: u8, on: bool },
    SetPwm { channel: u8, duty: u16 },
    Read { addr: u32 },
    Write { addr: u32, value: u32 },
}
// ANCHOR_END: naive

// ANCHOR: window
/// Окно периферии, к которому разрешён доступ по протоколу. Адреса регистров
/// выровнены на 4 байта, поэтому внутри окна их ровно 65 536.
pub const WINDOW_BASE: u32 = 0x4000_0000;
pub const WINDOW_LEN: u32 = 0x0004_0000;
// ANCHOR_END: window

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_is_twelve_bytes() {
        assert_eq!(core::mem::size_of::<NaiveCmd>(), 12);
        assert_eq!(WINDOW_LEN / 4, 1 << 16);
    }
}
