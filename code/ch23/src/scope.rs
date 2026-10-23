//! Раздел 23.5: где виден макрос.
//!
//! `macro_rules!` подчиняется *текстовой* области видимости: макрос виден
//! ниже своего определения, в том же модуле и во вложенных, объявленных
//! после него. До определения его нет:
//!
//! ```compile_fail
//! fn main() {
//!     early!(); // cannot find macro `early` in this scope
//! }
//! macro_rules! early {
//!     () => {};
//! }
//! ```
//!
//! `#[macro_export]` кладёт макрос в корень крейта, где бы он ни был
//! определён: путь к нему — `ch23::kib!`, а не `ch23::scope::kib!`.
//!
//! ```compile_fail,E0433
//! let _ = ch23::scope::kib!(4); // cannot find `kib` in `scope`
//! ```
//!
//! ```
//! assert_eq!(ch23::kib!(4), 4096);
//! ```
//!
//! Экспортированный макрос импортируется, как функция, — `#[macro_use]
//! extern crate` из редакции 2015 не нужен:
//!
//! ```
//! use ch23::kib;
//! assert_eq!(kib!(1), 1024);
//! ```
//!
//! Способ редакции 2015 по-прежнему работает (`standalone_crate`: такой
//! пример rustdoc не может слить с остальными в один крейт — `extern crate`
//! с макросами допустим только в корне):
//!
//! ```standalone_crate
//! #[macro_use]
//! extern crate ch23;
//!
//! fn main() {
//!     assert_eq!(kib!(2), 2048);
//! }
//! ```
//!
//! Неэкспортированный макрос можно сделать доступным по пути только внутри
//! крейта: `pub use` выдал бы его наружу, а это право есть только у
//! `#[macro_export]`.
//!
//! ```compile_fail,E0364
//! pub mod regs {
//!     macro_rules! reg_addr {
//!         ($base:expr, $off:expr) => { $base + $off };
//!     }
//!     pub use reg_addr; // only public within the crate, and cannot be re-exported outside
//! }
//! ```

// ANCHOR: textual
pub mod early {
    macro_rules! twice {
        ($e:expr) => {
            $e * 2
        };
    }

    pub fn four() -> u32 {
        twice!(2)
    }

    /// Вложенный модуль, объявленный после макроса, тоже его видит.
    pub mod nested {
        pub fn six() -> u32 {
            twice!(3)
        }
    }
}

/// `#[macro_use]` на модуле продлевает видимость его макросов за конец
/// модуля — до конца родителя.
#[macro_use]
mod up {
    macro_rules! triple {
        ($e:expr) => {
            $e * 3
        };
    }
}

pub fn nine() -> u32 {
    triple!(3)
}
// ANCHOR_END: textual

// ANCHOR: path_based
mod regs {
    macro_rules! reg_addr {
        ($base:expr, $off:expr) => {
            $base + $off
        };
    }

    /// После `use` макрос доступен по пути, как функция или тип.
    pub(crate) use reg_addr;
}

pub fn uart0_fr() -> usize {
    regs::reg_addr!(0x4000_C000, 0x18)
}
// ANCHOR_END: path_based

// ANCHOR: export
/// Килобайты в байты. Экспортирован — значит, доступен другим крейтам по
/// пути `ch23::kib!`.
#[macro_export]
macro_rules! kib {
    ($n:expr) => {
        $n * 1024
    };
}
// ANCHOR_END: export

#[cfg(test)]
mod tests {
    /// Повторное определение затеняет прежнее ниже по тексту, как `let`.
    #[test]
    fn later_definition_shadows() {
        macro_rules! version {
            () => {
                1
            };
        }
        let a = version!();
        macro_rules! version {
            () => {
                2
            };
        }
        let b = version!();
        assert_eq!((a, b), (1, 2));
    }

    #[test]
    fn scopes() {
        assert_eq!(super::early::four(), 4);
        assert_eq!(super::early::nested::six(), 6);
        assert_eq!(super::nine(), 9);
        assert_eq!(super::uart0_fr(), 0x4000_C018);
        assert_eq!(crate::kib!(64), 65536);
    }
}
