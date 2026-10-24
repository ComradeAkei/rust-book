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

// ANCHOR: verdicts
/// Ответы по фрагментам 1–6.
pub fn verdicts() -> [Verdict; 6] {
    use Bug::*;
    use InRust::*;
    [
        // 1: при n > 64 выход без kfree. В Rust CString и KVec — владельцы:
        // `return Err` роняет их сам.
        Verdict { bug: LeakOnError, rust: Gone },
        // 2: kfree, потом чтение. В Rust drop забирает вектор по значению,
        // и обращение к нему после — E0382.
        Verdict { bug: UseAfterFree, rust: CompileError },
        // 3: return -EAGAIN мимо mutex_unlock. В Rust мьютекс отпускает
        // guard при выходе из функции любым путём.
        Verdict { bug: MissingUnlock, rust: Gone },
        // 4: msleep под spin_lock. Типы не знают, в каком контексте
        // выполняется код: fsleep под SpinLock компилируется.
        Verdict { bug: SleepInAtomic, rust: Remains },
        // 5: проверка `i > n` пропускает i == n. В Rust индексация KVec
        // проверяет границу: паника вместо записи за конец.
        Verdict { bug: OutOfBounds, rust: Panic },
        // 6: порядок блокировок — свойство программы, а не типа: lockdep
        // ловит его в обоих языках.
        Verdict { bug: LockOrder, rust: Remains },
    ]
}
// ANCHOR_END: verdicts
