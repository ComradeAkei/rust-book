//! Раздел 23.4: рекурсия, внутренние правила, TT-muncher и накопители.
//!
//! Каждый рекурсивный вызов — уровень раскрытия, и уровней по умолчанию
//! не больше 128. Подсчёт рекурсией ломается на 128-м аргументе:
//!
//! ```compile_fail
//! macro_rules! count_rec {
//!     () => { 0usize };
//!     ($head:tt $($tail:tt)*) => { 1usize + count_rec!($($tail)*) };
//! }
//! // 128 аргументов: recursion limit reached while expanding `count_rec!`
//! const N: usize = count_rec!(
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//! );
//! ```
//!
//! Предел поднимает атрибут крейта — у того, *кто вызывает* макрос:
//!
//! ```
//! #![recursion_limit = "256"]
//! macro_rules! count_rec {
//!     () => { 0usize };
//!     ($head:tt $($tail:tt)*) => { 1usize + count_rec!($($tail)*) };
//! }
//! const N: usize = count_rec!(
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//!     x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x x
//! );
//! assert_eq!(N, 128);
//! ```
//!
//! Макрос не может раскрыться в ветку `match` — только в выражение,
//! образец, тип, оператор или элементы. Ветки собирают в накопитель и
//! выдают весь `match` разом (так устроен `fsm!` из задачи D):
//!
//! ```compile_fail
//! macro_rules! arm {
//!     ($p:pat => $e:expr) => { $p => $e };
//! }
//! fn f(x: u8) -> u8 {
//!     match x { arm!(0 => 1), _ => 2 } // macros cannot expand to match arms
//! }
//! ```

// Макросы раздела вызываются из тестов ниже.
#![cfg_attr(not(test), allow(unused_macros))]

// ANCHOR: reverse
/// Разворот списка. Повторение идёт только вперёд, поэтому нужен
/// накопитель `[...]`: на каждом шаге первый токен входа переезжает в
/// начало накопителя. `@acc` — внутреннее правило: снаружи его не зовут,
/// `@` лишь не даёт ему совпасть с обычным вызовом.
macro_rules! reverse {
    (@acc [$($done:tt)*]) => {
        [$($done),*]
    };
    (@acc [$($done:tt)*] $head:tt $($rest:tt)*) => {
        reverse!(@acc [$head $($done)*] $($rest)*)
    };
    ($($items:tt)*) => {
        reverse!(@acc [] $($items)*)
    };
}
// ANCHOR_END: reverse

// ANCHOR: fields
/// Поля регистра шириной в битах, `_` — зарезервированные биты. Смещение
/// поля — сумма ширин перед ним. Складывать числа макрос не умеет, но
/// умеет копить *выражение* `0 + 1 + 1` и отдать его компилятору.
///
/// Это TT-muncher: каждое правило «откусывает» одно поле от входа, и
/// поля бывают двух видов, чего одно повторение не выразит.
macro_rules! fields {
    // Вход съеден: ширины в сумме должны дать ширину регистра.
    (@munch $name:ident $ty:ty, [$($off:tt)*]) => {
        const _: () = assert!(
            $($off)* == <$ty>::BITS,
            "сумма ширин полей не равна ширине регистра"
        );
    };
    // Зарезервированные биты: только сдвинуть смещение.
    (@munch $name:ident $ty:ty, [$($off:tt)*] _: $w:literal $(, $($rest:tt)*)?) => {
        fields!(@munch $name $ty, [$($off)* + $w] $($($rest)*)?);
    };
    // Именованное поле: метод со смещением, накопленным до него.
    (@munch $name:ident $ty:ty, [$($off:tt)*] $field:ident: $w:literal $(, $($rest:tt)*)?) => {
        impl $name {
            pub const fn $field(self) -> $ty {
                (self.0 >> ($($off)*)) & ((1 << $w) - 1)
            }
        }
        fields!(@munch $name $ty, [$($off)* + $w] $($($rest)*)?);
    };
    ($(#[$m:meta])* $vis:vis struct $name:ident($ty:ty) { $($body:tt)* }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis struct $name(pub $ty);
        fields!(@munch $name $ty, [0] $($body)*);
    };
}

fields! {
    /// Регистр управления PL011 (UARTCR, раздел 6.9), младшие 16 бит.
    pub struct UartCr(u32) {
        uarten: 1, siren: 1, sirlp: 1, _: 4, lbe: 1,
        txe: 1, rxe: 1, dtr: 1, rts: 1, _: 2, rtsen: 1, ctsen: 1, _: 16,
    }
}
// ANCHOR_END: fields

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_list() {
        assert_eq!(reverse!(1 2 3), [3, 2, 1]);
        assert_eq!(reverse!("a"), ["a"]);
    }

    #[test]
    fn register_fields() {
        // UARTEN | TXE | RXE — значение, которое пишет драйвер главы 6.
        let cr = UartCr(0x301);
        assert_eq!((cr.uarten(), cr.txe(), cr.rxe()), (1, 1, 1));
        assert_eq!((cr.lbe(), cr.siren(), cr.rts()), (0, 0, 0));
        assert_eq!(UartCr(1 << 15).ctsen(), 1);
        assert_eq!(UartCr(1 << 14).rtsen(), 1);
    }
}
