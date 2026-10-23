//! Раздел 23.3: гигиена и `$crate`.
//!
//! Локальные переменные, метки и `$crate` из тела макроса разрешаются там,
//! где макрос определён; всё остальное — функции, типы, модули, макросы —
//! там, где он вызван (mixed-site hygiene). Поэтому макрос не видит
//! локальную переменную вызывающего, если её не передали аргументом:
//!
//! ```compile_fail,E0425
//! macro_rules! plus_one {
//!     () => { x + 1 };
//! }
//! let x = 1;
//! let _ = plus_one!(); // `x` в теле макроса — не этот `x`
//! ```
//!
//! А элементы негигиеничны: функция из макроса видна вызывающему, и
//! второй вызов того же макроса в том же модуле — повторное определение:
//!
//! ```compile_fail,E0428
//! macro_rules! make_helper {
//!     () => { fn helper() -> u32 { 42 } };
//! }
//! make_helper!();
//! make_helper!(); // the name `helper` is defined multiple times
//! ```

// Макросы раздела вызываются из тестов ниже.
#![cfg_attr(not(test), allow(unused_macros))]

// ANCHOR: swap
/// `tmp` из тела макроса — не тот `tmp`, что у вызывающего: у имени есть
/// не только текст, но и контекст раскрытия.
macro_rules! swap {
    ($a:ident, $b:ident) => {
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}
// ANCHOR_END: swap

// ANCHOR: items
/// Функция и тип из макроса видны там, где он вызван: гигиена их не
/// прячет. Имена задаёт вызывающий — так не будет столкновений.
macro_rules! sensor {
    ($ty:ident, $read:ident, $value:expr) => {
        #[derive(Debug, PartialEq)]
        pub struct $ty(pub u16);

        pub fn $read() -> $ty {
            $ty($value)
        }
    };
}

sensor!(Temperature, read_temperature, 215);
sensor!(Humidity, read_humidity, 40);
// ANCHOR_END: items

// ANCHOR: paths
/// Путь `Option::Some` разрешается у вызывающего: свой тип с тем же
/// именем молча подменит стандартный.
macro_rules! some_fragile {
    ($e:expr) => {
        Option::Some($e)
    };
}

/// Полный путь от корня: `::core` — внешний крейт, его не подменить ни
/// модулем `core`, ни типом `Option`.
macro_rules! some_robust {
    ($e:expr) => {
        ::core::option::Option::Some($e)
    };
}
// ANCHOR_END: paths

/// Вызывающий завёл свой `Option` — так бывает в коде протоколов.
pub mod shadow {
    #[derive(Debug, PartialEq)]
    pub enum Option<T> {
        Some(T),
        Absent,
    }

    pub fn fragile() -> Option<u8> {
        some_fragile!(1) // свой `Option`, а не стандартный
    }

    pub fn robust() -> ::core::option::Option<u8> {
        some_robust!(1)
    }

    pub fn absent() -> Option<u8> {
        Option::Absent
    }
}

// ANCHOR: dollar_crate
/// Помощник экспортированного макроса: публичный, чтобы его было видно из
/// чужого крейта, но скрытый из документации.
#[doc(hidden)]
pub fn __checked_add(a: u32, b: u32) -> u32 {
    a.checked_add(b).expect("переполнение в add_checked!")
}

/// `$crate` — путь к крейту, где определён макрос, как бы его ни назвали
/// у вызывающего (`ch23`, `extern crate ch23 as book`).
#[macro_export]
macro_rules! add_checked {
    ($a:expr, $b:expr) => {
        $crate::hygiene::__checked_add($a, $b)
    };
}
// ANCHOR_END: dollar_crate

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::manual_swap)] // ручной обмен — предмет примера
    fn swap_with_variable_named_tmp() {
        let mut x = 1;
        let mut tmp = 2;
        swap!(x, tmp);
        assert_eq!((x, tmp), (2, 1));
    }

    #[test]
    fn items_are_visible() {
        assert_eq!(read_temperature(), Temperature(215));
        assert_eq!(read_humidity(), Humidity(40));
    }

    #[test]
    fn local_defined_before_macro_is_visible() {
        // Макрос, определённый *после* `let`, видит эту переменную: имя
        // разрешается в месте определения.
        let base = 10;
        macro_rules! from_base {
            ($e:expr) => {
                base + $e
            };
        }
        assert_eq!(from_base!(5), 15);
    }

    #[test]
    fn fragile_path_is_captured() {
        assert_eq!(shadow::fragile(), shadow::Option::Some(1));
        assert_ne!(shadow::fragile(), shadow::absent());
        assert_eq!(shadow::robust(), Some(1));
    }

    #[test]
    fn dollar_crate_path() {
        assert_eq!(crate::add_checked!(2, 3), 5);
    }

    #[test]
    #[should_panic(expected = "переполнение")]
    fn dollar_crate_checks() {
        let _ = crate::add_checked!(u32::MAX, 1);
    }
}
