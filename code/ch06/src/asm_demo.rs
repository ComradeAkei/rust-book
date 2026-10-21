//! Функции для разделов «Под капотом». Каждая помечена `#[inline(never)]`
//! и `#[unsafe(no_mangle)]`, чтобы её было легко найти в выводе
//! `cargo asm`, `llvm-objdump` и `nm` под любую из целей книги.
//!
//! ```text
//! cargo asm -p ch06 --lib ch06_value_c
//! cargo asm -p ch06 --lib --target thumbv7m-none-eabi ch06_value_packed
//! cargo asm -p ch06 --lib --target thumbv6m-none-eabi ch06_value_packed
//! ```

use crate::enums::Message;
use crate::layout::SampleC;
use crate::packed::WireSample;
use core::num::NonZeroU32;

// ANCHOR: field_access
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_value_c(s: &SampleC) -> u32 {
    s.value
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_value_packed(s: &WireSample) -> u32 {
    s.value
}
// ANCHOR_END: field_access

// ANCHOR: niche_asm
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_unwrap_or_zero(x: Option<NonZeroU32>) -> u32 {
    match x {
        Some(v) => v.get(),
        None => 0,
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_is_some_ref(x: Option<&u8>) -> bool {
    x.is_some()
}
// ANCHOR_END: niche_asm

// ANCHOR: match_asm
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch06_describe(m: &Message) -> u32 {
    crate::enums::describe(m)
}
// ANCHOR_END: match_asm

// ANCHOR: repr_c_tag
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_is_pressure_c(m: &crate::enums::MessageC) -> bool {
    matches!(m, crate::enums::MessageC::Pressure(_))
}
// ANCHOR_END: repr_c_tag

// ANCHOR: u64_packed
/// 8-байтовое поле по смещению 1: проверяем, какие инструкции выберет LLVM
/// там, где двойная загрузка (`ldrd`) требует выравнивания.
#[repr(C, packed)]
pub struct Stamp {
    pub kind: u8,
    pub ticks: u64,
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_ticks(s: &Stamp) -> u64 {
    s.ticks
}

#[repr(C)]
pub struct StampAligned {
    pub kind: u8,
    pub ticks: u64,
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_ticks_aligned(s: &StampAligned) -> u64 {
    s.ticks
}
// ANCHOR_END: u64_packed

// ANCHOR: union_asm
// Начиная с Rust 1.75 крошечные функции вроде `unions::bits_safe` помечаются
// как встраиваемые между крейтами и не попадают в объектный файл своего
// крейта, пока их никто не вызвал. Обёртки ниже заставляют их появиться.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_bits_union(x: f32) -> u32 {
    crate::unions::bits_via_union(x)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn ch06_bits_safe(x: f32) -> u32 {
    crate::unions::bits_safe(x)
}
// ANCHOR_END: union_asm

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_functions_behave() {
        assert_eq!(ch06_bits_union(-2.0), ch06_bits_safe(-2.0));
        assert_eq!(ch06_unwrap_or_zero(NonZeroU32::new(9)), 9);
        assert_eq!(ch06_unwrap_or_zero(None), 0);
        assert!(ch06_is_some_ref(Some(&1)));
        let s = Stamp { kind: 1, ticks: 0x0102_0304_0506_0708 };
        assert_eq!(ch06_ticks(&s), 0x0102_0304_0506_0708);
        let a = StampAligned { kind: 1, ticks: 5 };
        assert_eq!(ch06_ticks_aligned(&a), 5);
        let c = SampleC { channel: 1, value: 2, flags: 3 };
        assert_eq!(ch06_value_c(&c), 2);
        let w = crate::packed::parse(&[0, 1, 0, 0, 0, 0]);
        assert_eq!(ch06_value_packed(&w), 1);
        assert_eq!(ch06_describe(&Message::Pressure(77)), 77);
        assert!(ch06_is_pressure_c(&crate::enums::MessageC::Pressure(1)));
        assert!(!ch06_is_pressure_c(&crate::enums::MessageC::Heartbeat));
    }
}
