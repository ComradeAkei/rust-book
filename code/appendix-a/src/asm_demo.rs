//! Функции для врезок «Под капотом» приложения A. Как и в главе 6, каждая
//! помечена `#[inline(never)]` и `#[unsafe(no_mangle)]`:
//!
//! ```text
//! cargo asm -p appendix-a --lib appa_quot
//! cargo asm -p appendix-a --lib --no-default-features --target thumbv7m-none-eabi appa_clz
//! ```

// ANCHOR: quot_asm
#[inline(never)]
#[unsafe(no_mangle)]
pub fn appa_quot(a: i32, b: i32) -> i32 {
    a / b
}
// ANCHOR_END: quot_asm

// ANCHOR: clz_asm
#[inline(never)]
#[unsafe(no_mangle)]
pub fn appa_clz(x: u32) -> u32 {
    x.leading_zeros()
}
// ANCHOR_END: clz_asm
