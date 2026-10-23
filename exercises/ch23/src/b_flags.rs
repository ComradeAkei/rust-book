//! Задача B: макрос в духе `bitflags` для флагов регистра.
//!
//! Образец уже разбирает вход и объявляет тип с константами флагов.
//! Допишите остальное: методы (`const fn` — их должно быть можно звать в
//! константах), операторы `|`, `&`, `!`, `|=`, `&=` и `Debug` вида
//! `Cr(UARTEN | TXE)`, `Cr(empty)`, а для неизвестных битов —
//! `Cr(TXE | 0x4)`. `!` — дополнение в пределах объявленных флагов.
//!
//! Макрос раскрывается в чужом крейте: пишите полные пути от `::core`.
//! Тест `hostile` проверит это на модуле со своими `Option` и `core`.
//! В `const fn` нельзя `todo!()` — заглушки там на `panic!`.

// ANCHOR: skeleton
#[macro_export]
macro_rules! flags {
    (
        $(#[$outer:meta])*
        $vis:vis struct $name:ident: $ty:ty {
            $(
                $(#[$inner:meta])*
                const $flag:ident = $value:expr;
            )*
        }
    ) => {
        $(#[$outer])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        $vis struct $name($ty);

        #[allow(dead_code)]
        impl $name {
            $(
                $(#[$inner])*
                pub const $flag: Self = Self($value);
            )*

            pub const fn empty() -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            /// Все объявленные биты.
            pub const fn all() -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub const fn bits(self) -> $ty {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            /// `None`, если есть биты, которых нет ни в одном флаге.
            pub const fn from_bits(bits: $ty) -> ::core::option::Option<Self> {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            /// Лишние биты отбрасываются.
            pub const fn from_bits_truncate(bits: $ty) -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            /// Как есть, с зарезервированными битами.
            pub const fn from_bits_retain(bits: $ty) -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub const fn is_empty(self) -> bool {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub const fn contains(self, other: Self) -> bool {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub const fn intersects(self, other: Self) -> bool {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub const fn union(self, other: Self) -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub const fn intersection(self, other: Self) -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub const fn difference(self, other: Self) -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            /// Дополнение в пределах объявленных флагов.
            pub const fn complement(self) -> Self {
                ::core::panic!("не решено: раздел 23.7, задача B")
            }

            pub fn insert(&mut self, other: Self) {
                ::core::todo!("раздел 23.7, задача B")
            }

            pub fn remove(&mut self, other: Self) {
                ::core::todo!("раздел 23.7, задача B")
            }
        }

        impl ::core::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                ::core::todo!("раздел 23.7, задача B")
            }
        }

        impl ::core::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self {
                ::core::todo!("раздел 23.7, задача B")
            }
        }

        impl ::core::ops::Not for $name {
            type Output = Self;
            fn not(self) -> Self {
                ::core::todo!("раздел 23.7, задача B")
            }
        }

        impl ::core::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, rhs: Self) {
                ::core::todo!("раздел 23.7, задача B")
            }
        }

        impl ::core::ops::BitAndAssign for $name {
            fn bitand_assign(&mut self, rhs: Self) {
                ::core::todo!("раздел 23.7, задача B")
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::todo!("раздел 23.7, задача B")
            }
        }
    };
}
// ANCHOR_END: skeleton
