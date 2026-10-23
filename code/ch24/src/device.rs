//! 24.4: `#[interrupt]` — обработчик прерывания в духе cortex-m-rt.
//!
//! Крейт устройства (в настоящей прошивке его генерирует svd2rust)
//! экспортирует под одним именем `interrupt` и перечисление прерываний, и
//! атрибут: у типов и макросов разные пространства имён. Одна строка
//! `use ch24::device::interrupt;` даёт пользователю оба.
//!
//! Имя обработчика сверяется со списком — E0599 с указанием на имя:
//!
//! ```compile_fail,E0599
//! use ch24::device::interrupt;
//! #[interrupt]
//! fn UART9() {}
//! ```
//!
//! Сигнатуру проверяет сам макрос, и ошибки указывают в неё:
//!
//! ```compile_fail
//! use ch24::device::interrupt;
//! #[interrupt]
//! async fn UART1(byte: u8) -> u8 { byte }
//! ```
//!
//! Атрибут можно назвать и полным путём, но путь `interrupt::UART1` из
//! его раскрытия разрешается в модуле пользователя — без `use` там нет
//! `interrupt`, и ошибка E0433 (так же ведёт себя cortex-m-rt):
//!
//! ```compile_fail,E0433
//! #[ch24::device::interrupt]
//! fn UART1() {}
//! ```

// ANCHOR: device
/// Внешние прерывания lm3s6965 (часть списка; номера — из таблицы 2-9
/// технического описания, как в крейте lm3s6965 0.2.0).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Interrupt {
    GPIOA = 0,
    UART0 = 5,
    UART1 = 6,
    TIMER_0A = 19,
}

pub use self::Interrupt as interrupt;
pub use ch24_macros::interrupt;
// ANCHOR_END: device

// ANCHOR: handler
use core::sync::atomic::{AtomicU32, Ordering};

/// Сколько раз вызван обработчик UART0.
pub static UART0_CALLS: AtomicU32 = AtomicU32::new(0);

#[interrupt]
fn UART0() {
    static mut COUNT: u32 = 0;
    // Здесь COUNT — уже `&mut u32`: макрос превратил static mut в
    // аргумент, и unsafe не нужен.
    *COUNT += 1;
    UART0_CALLS.store(*COUNT, Ordering::Relaxed);
}
// ANCHOR_END: handler

#[cfg(test)]
mod tests {
    use super::{UART0_CALLS, interrupt};
    use core::sync::atomic::Ordering;

    #[interrupt]
    fn TIMER_0A() -> ! {
        panic!("TIMER_0A: таймер не запускали");
    }

    // ANCHOR: handler_test
    unsafe extern "C" {
        // Символ, под которым макрос экспортировал переходник; в прошивке
        // его адрес кладёт в таблицу векторов крейт устройства.
        fn UART0();
    }

    #[test]
    fn handler_keeps_state_between_calls() {
        for _ in 0..3 {
            // SAFETY: переходник вызывает только этот тест, по одному
            // разу за раз, — как процессор, который не входит в
            // обработчик повторно.
            unsafe { UART0() };
        }
        assert_eq!(UART0_CALLS.load(Ordering::Relaxed), 3);
    }
    // ANCHOR_END: handler_test

    #[test]
    fn diverging_handler_compiles() {
        let _: unsafe extern "C" fn() = __ch24_TIMER_0A_trampoline;
    }
}
