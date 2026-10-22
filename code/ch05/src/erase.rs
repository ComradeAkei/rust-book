//! Раздел 5.3: времена жизни стираются при компиляции.

// ANCHOR: erase
/// Две функции различаются только временами жизни. Машинный код у них
/// один и тот же, и LLVM оставляет одну копию, а второе имя делает
/// псевдонимом первого.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch05_first<'a>(x: &'a [u8]) -> Option<&'a u8> {
    x.first()
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch05_first_static(x: &'static [u8]) -> Option<&'static u8> {
    x.first()
}
// ANCHOR_END: erase

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlv::Tlv;
    use core::any::type_name;

    #[test]
    fn erased() {
        static FRAME: [u8; 3] = [0x7E, 1, 2];
        let local = [0x55u8];
        assert_eq!(ch05_first(&local), Some(&0x55));
        assert_eq!(ch05_first_static(&FRAME), Some(&0x7E));
        // Даже имя типа во время выполнения не помнит, какое было время
        // жизни: на его месте `'_` или ничего.
        assert_eq!(type_name::<&'static str>(), "&str");
        assert_eq!(type_name::<Tlv<'static>>(), "ch05::tlv::Tlv<'_>");
        assert_eq!(type_name::<Tlv<'static>>(), type_name::<Tlv<'_>>());
    }
}
