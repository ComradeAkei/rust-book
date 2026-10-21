//! Задача 6.D. Команды протокола в 8 байтах.
//!
//! Исходные данные в `code/ch06/src/task_d.rs`: перечисление `NaiveCmd`
//! (12 байт) и окно периферии `WINDOW_BASE..WINDOW_BASE + WINDOW_LEN`.
//! Команды `Read` и `Write` допустимы только для адресов внутри окна,
//! выровненных на 4.
//!
//! Требования, которые проверяют тесты:
//! - `size_of::<Cmd>() == 8`;
//! - `size_of::<Option<Cmd>>() == 8`;
//! - `size_of::<Result<Cmd, ConvertError>>() == 8`, и при этом ошибка
//!   сообщает вид нарушения и исходный адрес целиком;
//! - любая допустимая `NaiveCmd` переживает путь `NaiveCmd → Cmd → NaiveCmd`;
//! - те же тесты проходят под `-Zrandomize-layout` (раздел 6.2): 8 байт не
//!   должны зависеть от того, как rustc расставит поля.
//!
//! Исследовательская часть (письменно): составьте таблицу из 10 и более
//! типов, где компилятор находит нишу, и стольких же, где не находит, и для
//! каждого объясните причину. Начните с вопроса, почему исходный вариант
//! `ConvertError` ниже не проходит третий тест.

use ch06::task_d::{NaiveCmd, WINDOW_BASE, WINDOW_LEN};

/// Спроектируйте варианты. Подсказка: сколько разных адресов на самом деле
/// может прийти в `Read` и `Write`?
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Todo,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ConvertErrorKind {
    OutOfWindow,
    Misaligned,
}

/// Первая версия ошибки. Она работает, но раздувает `Result<Cmd, _>`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConvertError {
    kind: ConvertErrorKind,
    addr: u32,
}

impl ConvertError {
    pub const fn kind(&self) -> ConvertErrorKind {
        self.kind
    }
    pub const fn addr(&self) -> u32 {
        self.addr
    }
}

impl TryFrom<NaiveCmd> for Cmd {
    type Error = ConvertError;

    fn try_from(c: NaiveCmd) -> Result<Self, ConvertError> {
        todo!()
    }
}

impl From<Cmd> for NaiveCmd {
    fn from(c: Cmd) -> Self {
        todo!()
    }
}
