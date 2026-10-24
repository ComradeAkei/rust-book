//! Задача 29.A: шесть фрагментов драйвера датчика на C
//! (`tests-shared/ch29/a/c/ch29_aN.c`), в каждом по ошибке, и их переводы
//! на Rust с абстракциями крейта `kernel` (`tests-shared/ch29/a/rust/aN`).
//! Тест проверяет ответы опытом: собирает переводы и грузит каждый модуль
//! в отладочное ядро (KASAN, kmemleak, lockdep).

/// Ошибка во фрагменте на C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bug {
    /// Память не освобождается на пути ошибки.
    LeakOnError,
    /// Обращение к памяти после kfree.
    UseAfterFree,
    /// Возврат с захваченным мьютексом.
    MissingUnlock,
    /// Сон под спин-блокировкой.
    SleepInAtomic,
    /// Запись за конец массива.
    OutOfBounds,
    /// Две блокировки берутся в разном порядке (ABBA).
    LockOrder,
}

/// Что стало с ошибкой в переводе на Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InRust {
    /// Перевод не компилируется.
    CompileError,
    /// Компилируется, и ошибки нет: её исключили владение и Drop.
    Gone,
    /// Компилируется; ошибку ловит проверка во время выполнения — паника,
    /// то есть oops вместо порчи памяти.
    Panic,
    /// Компилируется, и ошибка осталась: отладочное ядро сообщает о ней
    /// так же, как о версии на C.
    Remains,
}

/// Ответ по одному фрагменту.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    pub bug: Bug,
    pub rust: InRust,
}

/// Ответы по фрагментам 1–6: для каждого — какая ошибка в версии на C
/// и что с ней стало в переводе на Rust.
pub fn verdicts() -> [Verdict; 6] {
    todo!("29.A: прочтите фрагменты tests-shared/ch29/a/c и их переводы tests-shared/ch29/a/rust")
}
