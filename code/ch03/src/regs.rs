//! Задача 3.B: регистровый файл периферии с журналом записей.
//!
//! Модуль без `std`: шестнадцать 32-битных регистров и история последних
//! записей, по которой тесты проверяют, в каком порядке транзакция
//! откатывает изменения.

/// Число регистров.
pub const NREGS: usize = 16;
/// Сколько записей помнит история.
pub const HISTORY: usize = 64;

// ANCHOR: regs
/// Регистры периферии. Каждая запись попадает в историю: на настоящем
/// устройстве порядок записей важен не меньше, чем их значения.
pub struct Regs {
    words: [u32; NREGS],
    history: [(u8, u32); HISTORY],
    len: usize,
}

impl Regs {
    pub const fn new() -> Self {
        Regs { words: [0; NREGS], history: [(0, 0); HISTORY], len: 0 }
    }

    pub fn read(&self, i: usize) -> u32 {
        self.words[i]
    }

    pub fn write(&mut self, i: usize, value: u32) {
        self.words[i] = value;
        if self.len < HISTORY {
            self.history[self.len] = (i as u8, value);
            self.len += 1;
        }
    }

    /// Все записи по порядку: номер регистра и записанное значение.
    pub fn history(&self) -> &[(u8, u32)] {
        &self.history[..self.len]
    }
}
// ANCHOR_END: regs

impl Default for Regs {
    fn default() -> Self {
        Self::new()
    }
}
