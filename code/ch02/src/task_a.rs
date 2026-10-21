//! Задача 2.A: пять `match`. Для каждого предскажите, во что LLVM
//! превратит его в release-сборке для x86_64, и только потом проверьте по
//! ассемблеру.

/// Способы, которыми `match` становится машинным кодом (раздел 2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lowering {
    /// Косвенный переход по таблице адресов ветвей.
    JumpTable,
    /// Значение читается из таблицы в памяти.
    MemoryTable,
    /// Значение извлекается сдвигом из константы в регистре.
    RegisterTable,
    /// Значение вычисляется формулой, без таблиц и переходов по веткам.
    Formula,
    /// Цепочка или дерево сравнений с условными переходами.
    Compares,
}

// ANCHOR: list
/// Код команды флеш-памяти.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FlashCmd {
    Read = 1,
    Write = 2,
    Erase = 4,
    Reset = 8,
}

/// Уровень записи в журнале.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// 1. Метод HTTP по первой букве запроса.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02a_method(first: u8) -> u8 {
    match first {
        b'G' => 1, // GET
        b'P' => 2, // POST, PUT
        b'D' => 3, // DELETE
        b'H' => 4, // HEAD
        b'O' => 5, // OPTIONS
        b'C' => 6, // CONNECT
        b'T' => 7, // TRACE
        _ => 0,
    }
}

/// 2. Время выполнения команды флеш-памяти в микросекундах.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02a_flash_time(cmd: FlashCmd) -> u32 {
    match cmd {
        FlashCmd::Read => 1,
        FlashCmd::Write => 40,
        FlashCmd::Erase => 4000,
        FlashCmd::Reset => 20,
    }
}

/// 3. Номер регистра UART по его адресу.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02a_uart_reg(addr: u32) -> u32 {
    match addr {
        0x4000_C000 => 1, // DR
        0x4000_C004 => 2, // RSR
        0x4000_C008 => 3,
        0x4000_C00C => 4,
        0x4000_C010 => 5,
        _ => 0,
    }
}

/// 4. Число десятичных цифр.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02a_digits(x: u32) -> u32 {
    match x {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        1000..=9999 => 4,
        _ => 5,
    }
}

/// 5. Буква уровня для строки журнала.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02a_level_letter(level: Level) -> u8 {
    match level {
        Level::Error => b'E',
        Level::Warn => b'W',
        Level::Info => b'I',
        Level::Debug => b'D',
        Level::Trace => b'T',
    }
}
// ANCHOR_END: list

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        assert_eq!(ch02a_method(b'O'), 5);
        assert_eq!(ch02a_method(b'X'), 0);
        assert_eq!(ch02a_flash_time(FlashCmd::Erase), 4000);
        assert_eq!(ch02a_uart_reg(0x4000_C00C), 4);
        assert_eq!(ch02a_uart_reg(0x4000_C014), 0);
        assert_eq!(ch02a_uart_reg(0x4000_C001), 0);
        assert_eq!(ch02a_digits(0), 1);
        assert_eq!(ch02a_digits(9999), 4);
        assert_eq!(ch02a_digits(u32::MAX), 5);
        assert_eq!(ch02a_level_letter(Level::Debug), b'D');
    }
}
