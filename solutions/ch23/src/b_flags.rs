//! Задача 23.B: макрос в духе `bitflags` для флагов регистра.
//!
//! ```
//! ch23_solutions::flags! {
//!     /// Регистр управления PL011 (UARTCR), используемые биты.
//!     pub struct Cr: u32 {
//!         const UARTEN = 1 << 0;
//!         const TXE = 1 << 8;
//!         const RXE = 1 << 9;
//!     }
//! }
//! // Операторы — методы трейтов, в `const` их не вызвать; для констант
//! // есть `const fn union`.
//! const ON: Cr = Cr::UARTEN.union(Cr::TXE).union(Cr::RXE);
//! assert_eq!(ON.bits(), 0x301);
//! assert_eq!(format!("{ON:?}"), "Cr(UARTEN | TXE | RXE)");
//! assert_eq!(format!("{:?}", !Cr::TXE), "Cr(UARTEN | RXE)");
//! ```

// ANCHOR: solution
/// Тип-обёртка над целым и по константе на флаг. Все пути — от `::core`:
/// макрос раскрывается в чужом крейте, где `Option` или `fmt` могут
/// оказаться своими.
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

            /// Имена и значения флагов в порядке объявления — для `Debug`.
            pub const FLAGS: &'static [(&'static str, Self)] =
                &[$((::core::stringify!($flag), Self::$flag)),*];

            pub const fn empty() -> Self {
                Self(0)
            }

            /// Все объявленные биты: `0 | A | B | ...`.
            pub const fn all() -> Self {
                Self(0 $(| $value)*)
            }

            pub const fn bits(self) -> $ty {
                self.0
            }

            /// `None`, если есть биты, которых нет ни в одном флаге.
            pub const fn from_bits(bits: $ty) -> ::core::option::Option<Self> {
                if bits & !Self::all().0 == 0 {
                    ::core::option::Option::Some(Self(bits))
                } else {
                    ::core::option::Option::None
                }
            }

            /// Лишние биты отбрасываются.
            pub const fn from_bits_truncate(bits: $ty) -> Self {
                Self(bits & Self::all().0)
            }

            /// Как есть: регистр вправе вернуть зарезервированные биты.
            pub const fn from_bits_retain(bits: $ty) -> Self {
                Self(bits)
            }

            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }

            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            pub const fn intersects(self, other: Self) -> bool {
                self.0 & other.0 != 0
            }

            pub const fn union(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }

            pub const fn intersection(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }

            pub const fn difference(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }

            /// Дополнение в пределах объявленных флагов: `!` не включает
            /// зарезервированные биты регистра.
            pub const fn complement(self) -> Self {
                Self(!self.0 & Self::all().0)
            }

            pub fn insert(&mut self, other: Self) {
                self.0 |= other.0;
            }

            pub fn remove(&mut self, other: Self) {
                self.0 &= !other.0;
            }
        }

        impl ::core::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                self.union(rhs)
            }
        }

        impl ::core::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self {
                self.intersection(rhs)
            }
        }

        impl ::core::ops::Not for $name {
            type Output = Self;
            fn not(self) -> Self {
                self.complement()
            }
        }

        impl ::core::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, rhs: Self) {
                self.insert(rhs);
            }
        }

        impl ::core::ops::BitAndAssign for $name {
            fn bitand_assign(&mut self, rhs: Self) {
                *self = self.intersection(rhs);
            }
        }

        /// `Cr(UARTEN | TXE)`, `Cr(empty)`, а неизвестные биты — числом:
        /// `Cr(TXE | 0x4)`.
        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(::core::concat!(::core::stringify!($name), "("))?;
                let mut rest = self.0;
                let mut first = true;
                for &(name, flag) in Self::FLAGS {
                    if flag.0 != 0 && self.0 & flag.0 == flag.0 {
                        if !first {
                            f.write_str(" | ")?;
                        }
                        f.write_str(name)?;
                        rest &= !flag.0;
                        first = false;
                    }
                }
                if rest != 0 {
                    if !first {
                        f.write_str(" | ")?;
                    }
                    ::core::write!(f, "{:#x}", rest)?;
                    first = false;
                }
                if first {
                    f.write_str("empty")?;
                }
                f.write_str(")")
            }
        }
    };
}
// ANCHOR_END: solution
