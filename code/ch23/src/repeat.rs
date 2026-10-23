//! Раздел 23.2: повторения `$(...)*`, `+` и `?`, разделители, вложенные
//! повторения и подсчёт аргументов при компиляции.
//!
//! Переменные, захваченные вместе, повторяются в подстановке вместе, и
//! длины их повторений обязаны совпасть:
//!
//! ```compile_fail
//! macro_rules! zip {
//!     ([$($a:expr),*] [$($b:expr),*]) => { [$(($a, $b)),*] };
//! }
//! let _ = zip!([1, 2] [3]); // meta-variable `a` repeats 2 times, but `b` repeats 1 time
//! ```
//!
//! Метапеременная из внутреннего повторения не может стоять во внешнем —
//! там их много, и непонятно, какую подставить:
//!
//! ```compile_fail
//! macro_rules! flat {
//!     ($($periph:ident { $($reg:ident),* })*) => {
//!         [$(stringify!($reg)),*] // variable `reg` is still repeating at this depth
//!     };
//! }
//! let _ = flat!(UART0 { DR, FR });
//! ```
//!
//! Повторение `+` требует хотя бы одного элемента: `bits!()` — ошибка
//! вызова, а не тихий ноль.
//!
//! ```compile_fail
//! macro_rules! bits {
//!     ($($b:literal),+ $(,)?) => { 0 $(| 1 << $b)+ };
//! }
//! let _: u32 = bits!(); // unexpected end of macro invocation
//! ```
//!
//! Длина таблицы, вычисленная из списка имён, ловит рассинхронизацию: в
//! списке четыре датчика, а в таблице три строки.
//!
//! ```compile_fail,E0308
//! macro_rules! replace { ($_t:tt $sub:expr) => { $sub }; }
//! macro_rules! count { ($($x:tt)*) => { <[()]>::len(&[$(replace!($x ())),*]) }; }
//! const SENSORS: usize = count!(temp humidity pressure light);
//! static SENSOR_NAMES: [&str; SENSORS] = ["temp", "humidity", "pressure"];
//! // expected an array with a size of 4, found one with a size of 3
//! ```
//!
//! Встроенный счётчик повторений есть только на nightly: метавыражение
//! `${count(x)}` в rustc 1.97 нестабильно (issue #83527), поэтому `count!`
//! ниже считает сам.
//!
//! ```compile_fail,E0658
//! macro_rules! count {
//!     ($($x:ident)*) => { ${count($x)} }; // meta-variable expressions are unstable
//! }
//! let _ = count!(a b c);
//! ```

// Макросы раздела вызываются из тестов ниже.
#![cfg_attr(not(test), allow(unused_macros))]

// ANCHOR: sum
/// Ноль или больше выражений через запятую, запятая в конце — по желанию.
macro_rules! sum {
    ($($x:expr),* $(,)?) => {
        0 $(+ $x)*
    };
}

/// Хотя бы один номер бита (`+`): маска из пустого списка — ошибка
/// вызова, а не тихий ноль.
macro_rules! bits {
    ($($b:literal),+ $(,)?) => {
        0 $(| 1 << $b)+
    };
}
// ANCHOR_END: sum

// ANCHOR: optional
/// Структура настроек со значениями по умолчанию: `= значение` у поля —
/// по желанию (`?`). Внутреннее правило `@or` выбирает, что подставить.
macro_rules! config {
    (@or) => {
        ::core::default::Default::default()
    };
    (@or $e:expr) => {
        $e
    };
    ($vis:vis struct $name:ident { $($field:ident: $ty:ty $(= $default:expr)?),* $(,)? }) => {
        #[derive(Debug, PartialEq)]
        $vis struct $name {
            $(pub $field: $ty,)*
        }

        impl ::core::default::Default for $name {
            fn default() -> Self {
                Self { $($field: config!(@or $($default)?),)* }
            }
        }
    };
}

config! {
    pub struct UartConfig {
        baud: u32 = 115_200,
        data_bits: u8 = 8,
        parity: bool,
        stop_bits: u8 = 1,
    }
}
// ANCHOR_END: optional

// ANCHOR: nested
/// Карта регистров нескольких устройств: внешнее повторение — устройства,
/// внутреннее — их регистры. `$base` захвачен на внешнем уровне и
/// используется во внутреннем повторении — так можно; наоборот нельзя.
macro_rules! register_map {
    ($($periph:ident @ $base:literal { $($reg:ident @ $off:literal),* $(,)? })*) => {
        $(
            #[allow(non_snake_case)]
            pub mod $periph {
                pub const BASE: usize = $base;
                $(pub const $reg: usize = $base + $off;)*
            }
        )*

        /// Все регистры одним списком: имя и адрес.
        pub const ALL: &[(&str, usize)] = &[
            $($((concat!(stringify!($periph), ".", stringify!($reg)), $base + $off),)*)*
        ];
    };
}

register_map! {
    UART0 @ 0x4000_C000 { DR @ 0x000, FR @ 0x018, CR @ 0x030 }
    SYSTICK @ 0xE000_E010 { CSR @ 0x0, RVR @ 0x4, CVR @ 0x8 }
}
// ANCHOR_END: nested

// ANCHOR: count
/// Подменяет токен выражением: повторение идёт по аргументам, а
/// производит своё.
macro_rules! replace {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

/// Число аргументов как константа. Длина среза из `()` известна при
/// компиляции, рекурсии нет — годится и для тысяч аргументов.
macro_rules! count {
    ($($x:tt)*) => {
        <[()]>::len(&[$(replace!($x ())),*])
    };
}

/// Тот же подсчёт рекурсией: на каждый аргумент — уровень раскрытия,
/// и после 127 аргументов срабатывает предел рекурсии (раздел 23.4).
macro_rules! count_rec {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count_rec!($($tail)*) };
}

/// Длина массива из числа имён — то, что в C пишут как
/// `sizeof(names) / sizeof(names[0])`, только известно до компиляции
/// инициализатора.
pub const SENSORS: usize = count!(temp humidity pressure light);
pub static SENSOR_NAMES: [&str; SENSORS] = ["temp", "humidity", "pressure", "light"];
// ANCHOR_END: count

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_and_bits() {
        assert_eq!(sum!(), 0);
        assert_eq!(sum!(1, 2, 3,), 6);
        assert_eq!(bits!(0, 8, 9), 0x301); // UARTEN | TXE | RXE
    }

    #[test]
    fn config_defaults() {
        let c = UartConfig::default();
        assert_eq!(c, UartConfig { baud: 115_200, data_bits: 8, parity: false, stop_bits: 1 });
    }

    #[test]
    fn nested_register_map() {
        assert_eq!(UART0::FR, 0x4000_C018);
        assert_eq!(SYSTICK::RVR, 0xE000_E014);
        assert_eq!(ALL.len(), 6);
        assert_eq!(ALL[1], ("UART0.FR", 0x4000_C018));
        assert_eq!(ALL[5], ("SYSTICK.CVR", 0xE000_E018));
    }

    #[test]
    fn counting_at_compile_time() {
        const N: usize = count!(a b c);
        let table = [0u8; count!(x y z w)];
        assert_eq!((N, table.len()), (3, 4));
        assert_eq!(count_rec!(a b c), 3);
        assert_eq!(count!(), 0);
        assert_eq!(SENSOR_NAMES.len(), 4);
    }
}
