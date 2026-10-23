//! Раздел 23.6: макросы для железа — X-macro, таблица обработчиков,
//! регистры и `unsafe` внутри макроса.
//!
//! Макрос, раскрывающийся в `unsafe`, не должен подставлять туда выражение
//! вызывающего: иначе тот выполнит небезопасную операцию, не написав слова
//! `unsafe`. Правильный [`take_once!`](crate::take_once) вычисляет
//! инициализатор снаружи блока, и разыменование сырого указателя в нём —
//! ошибка компиляции:
//!
//! ```compile_fail,E0133
//! let p = &5u32 as *const u32;
//! let _ = ch23::take_once!(u32 = *p); // dereference of raw pointer is unsafe
//! ```
//!
//! А [`take_once_bad!`](crate::take_once_bad) то же самое пропускает:
//!
//! ```
//! let p = &5u32 as *const u32;
//! let r = ch23::take_once_bad!(u32 = *p); // ни слова `unsafe` — и компилируется
//! assert_eq!(r.map(|v| *v), Some(5));
//! ```

use core::ptr::{read_volatile, write_volatile};

// ANCHOR: xmacro_list
/// Коды ошибок протокола станции — один список на всё. Сам макрос ничего
/// не генерирует: он отдаёт список макросу-генератору `$gen`, как
/// `#define ERRORS(X)` в C отдаёт его макросу `X`.
macro_rules! station_errors {
    ($gen:ident) => {
        $gen! {
            Ok = 0 => "нет ошибки",
            Timeout = 1 => "тайм-аут",
            Crc = 2 => "ошибка CRC",
            Overrun = 3 => "переполнение приёмника",
            Framing = 4 => "ошибка кадра",
        }
    };
}
// ANCHOR_END: xmacro_list

// ANCHOR: xmacro_enum
/// Первый генератор: перечисление, текст и разбор кода с провода.
macro_rules! gen_enum {
    ($($name:ident = $code:literal => $text:literal),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u8)]
        pub enum StationError {
            $($name = $code),*
        }

        impl StationError {
            pub const ALL: &'static [Self] = &[$(Self::$name),*];

            pub const fn text(self) -> &'static str {
                match self {
                    $(Self::$name => $text),*
                }
            }

            pub const fn from_code(code: u8) -> Option<Self> {
                match code {
                    $($code => Some(Self::$name),)*
                    _ => None,
                }
            }
        }
    };
}

station_errors!(gen_enum);
// ANCHOR_END: xmacro_enum

// ANCHOR: xmacro_header
/// Второй генератор из того же списка: заголовок для C-стороны провода.
/// Имена — как в списке: перевести `Timeout` в `TIMEOUT` `macro_rules!`
/// не умеет, это уже работа процедурного макроса (глава 24).
macro_rules! gen_c_header {
    ($($name:ident = $code:literal => $text:literal),* $(,)?) => {
        pub const C_HEADER: &str = concat!(
            "/* сгенерировано из station_errors! */\n",
            $("#define STATION_ERR_", stringify!($name), " ", stringify!($code),
              " /* ", $text, " */\n",)*
        );
    };
}

station_errors!(gen_c_header);
// ANCHOR_END: xmacro_header

// ANCHOR: handlers
/// Таблица обработчиков: перечисление источников, массив указателей на
/// функции в порядке номеров и диспетчер. Номер — индекс в массиве, так
/// что пропуск или повтор номера — ошибка компиляции, её ловит `const`.
macro_rules! irq_table {
    ($($irq:ident = $n:literal => $handler:path),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u8)]
        pub enum Irq {
            $($irq = $n),*
        }

        static HANDLERS: [fn(); [$($n),*].len()] = [$($handler),*];

        const _: () = {
            let n = [$($n),*];
            let mut i = 0;
            while i < n.len() {
                assert!(n[i] as usize == i, "номера идут подряд с нуля");
                i += 1;
            }
        };

        pub fn dispatch(irq: Irq) {
            HANDLERS[irq as usize]()
        }
    };
}
// ANCHOR_END: handlers

/// Обработчики оставляют след: номер последнего сработавшего. Только
/// load/store — собирается и для Cortex-M0.
pub static LAST: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(u8::MAX);

fn on_tick() {
    LAST.store(0, core::sync::atomic::Ordering::Relaxed);
}

fn on_uart() {
    LAST.store(1, core::sync::atomic::Ordering::Relaxed);
}

fn on_timer() {
    LAST.store(2, core::sync::atomic::Ordering::Relaxed);
}

irq_table! {
    SysTick = 0 => on_tick,
    Uart0 = 1 => on_uart,
    Timer0 = 2 => on_timer,
}

// ANCHOR: mmio
/// Регистры одним списком: на каждый — функции чтения и записи с
/// фиксированным адресом. Имена обеих задаёт вызывающий: склеить `set_`
/// с `dr` `macro_rules!` не может.
macro_rules! mmio {
    ($base:literal { $($read:ident / $write:ident @ $off:literal),* $(,)? }) => {
        $(
            #[inline(always)]
            pub fn $read() -> u32 {
                // SAFETY: адрес — регистр устройства этой машины; доступ
                // только volatile.
                unsafe { ::core::ptr::read_volatile(($base + $off) as *const u32) }
            }

            #[inline(always)]
            pub fn $write(v: u32) {
                // SAFETY: то же, что у чтения.
                unsafe { ::core::ptr::write_volatile(($base + $off) as *mut u32, v) }
            }
        )*
    };
}

/// UART0 машины lm3s6965evb (PL011).
pub mod uart0 {
    mmio! {
        0x4000_C000 {
            dr / set_dr @ 0x000,
            fr / set_fr @ 0x018,
            cr / set_cr @ 0x030,
        }
    }
}

const TXFF: u32 = 1 << 5;

/// Отправка байта через сгенерированные функции.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch23_macro_send(b: u8) {
    while uart0::fr() & TXFF != 0 {}
    uart0::set_dr(b as u32);
}

/// Та же отправка руками. Тест `rustc_macro_is_free` сравнивает обе
/// функции в сборке под thumbv7m-none-eabi.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch23_hand_send(b: u8) {
    // SAFETY: адреса регистров FR и DR UART0 этой машины.
    unsafe {
        while read_volatile(0x4000_C018 as *const u32) & TXFF != 0 {}
        write_volatile(0x4000_C000 as *mut u32, b as u32);
    }
}
// ANCHOR_END: mmio

// ANCHOR: unsafe_trap
/// Плохо: `$init` подставлен внутрь `unsafe`, и вызывающий выполняет там
/// что угодно — разыменование сырого указателя, вызов `unsafe fn`, — не
/// написав слова `unsafe`. Макрос делает вызывающий код незаметно
/// небезопасным.
#[macro_export]
macro_rules! take_once_bad {
    ($ty:ty = $init:expr) => {{
        static TAKEN: ::core::sync::atomic::AtomicBool =
            ::core::sync::atomic::AtomicBool::new(false);
        static mut SLOT: ::core::mem::MaybeUninit<$ty> = ::core::mem::MaybeUninit::uninit();
        if TAKEN.swap(true, ::core::sync::atomic::Ordering::AcqRel) {
            None
        } else {
            // SAFETY: TAKEN выдаёт SLOT ровно один раз.
            unsafe {
                let p = (&raw mut SLOT).cast::<$ty>();
                p.write($init);
                Some(&mut *p)
            }
        }
    }};
}

/// Хорошо: инициализатор вычисляется до `unsafe`, внутри — только своя
/// операция. Так устроен `cortex_m::singleton!`.
#[macro_export]
macro_rules! take_once {
    ($ty:ty = $init:expr) => {{
        static TAKEN: ::core::sync::atomic::AtomicBool =
            ::core::sync::atomic::AtomicBool::new(false);
        static mut SLOT: ::core::mem::MaybeUninit<$ty> = ::core::mem::MaybeUninit::uninit();
        if TAKEN.swap(true, ::core::sync::atomic::Ordering::AcqRel) {
            None
        } else {
            let init: $ty = $init;
            // SAFETY: TAKEN выдаёт SLOT ровно один раз.
            unsafe {
                let p = (&raw mut SLOT).cast::<$ty>();
                p.write(init);
                Some(&mut *p)
            }
        }
    }};
}
// ANCHOR_END: unsafe_trap

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::Ordering;

    #[test]
    fn xmacro_enum() {
        assert_eq!(StationError::ALL.len(), 5);
        assert_eq!(StationError::from_code(2), Some(StationError::Crc));
        assert_eq!(StationError::from_code(9), None);
        assert_eq!(StationError::Overrun.text(), "переполнение приёмника");
        assert_eq!(StationError::Framing as u8, 4);
    }

    #[test]
    fn xmacro_header() {
        assert!(C_HEADER.starts_with("/* сгенерировано из station_errors! */\n"));
        assert!(C_HEADER.contains("#define STATION_ERR_Timeout 1 /* тайм-аут */\n"));
        assert_eq!(C_HEADER.lines().count(), 6);
    }

    #[test]
    fn handler_table() {
        dispatch(Irq::Uart0);
        assert_eq!(LAST.load(Ordering::Relaxed), 1);
        dispatch(Irq::Timer0);
        assert_eq!(LAST.load(Ordering::Relaxed), 2);
        dispatch(Irq::SysTick);
        assert_eq!(LAST.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn take_once_gives_once() {
        let first = take_once!([u8; 4] = [1, 2, 3, 4]);
        let first = first.expect("первый раз — буфер");
        first[0] = 9;
        assert_eq!(*first, [9, 2, 3, 4]);
    }

    /// Clippy видит подстановку `$init` внутрь `unsafe`, но только там, где
    /// экспортированный макрос раскрыт: в самом крейте без вызова он молчит.
    /// `#[expect]` делает из этого проверку: если линт перестанет
    /// срабатывать, `cargo clippy -D warnings` упадёт на
    /// `unfulfilled_lint_expectations`.
    #[test]
    #[expect(clippy::macro_metavars_in_unsafe)]
    fn take_once_bad_is_flagged_by_clippy() {
        let r = take_once_bad!(u32 = 5);
        assert_eq!(r.map(|v| *v), Some(5));
    }

    #[test]
    fn take_once_second_time_none() {
        fn get() -> Option<&'static mut u32> {
            take_once!(u32 = 7)
        }
        assert_eq!(get().map(|v| *v), Some(7));
        assert!(get().is_none());
    }
}
