//! Раздел 17.5 в ассемблере: volatile запрещает сливать обращения. Два
//! обычных чтения одного адреса компилятор сливает в одно; два
//! volatile-чтения обязан сохранить оба. Снято `asm`-тестом главы.

// ANCHOR: plain
/// Два обычных чтения одного адреса — компилятор сольёт в одно.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch17_two_plain(p: &u32) -> u32 {
    let a = *p;
    let b = *p;
    a.wrapping_add(b)
}
// ANCHOR_END: plain

// ANCHOR: volatile
/// Два volatile-чтения того же адреса — оба обязаны остаться: у каждого
/// обращения к MMIO может быть побочный эффект.
///
/// # Safety
///
/// `p` — валидный, выровненный на 4 адрес, который можно читать дважды.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn ch17_two_volatile(p: *const u32) -> u32 {
    // SAFETY: предусловие функции переложено на вызывающего (asm-тест даёт
    // валидный адрес). volatile-чтения компилятор не сольёт.
    unsafe {
        let a = core::ptr::read_volatile(p);
        let b = core::ptr::read_volatile(p);
        a.wrapping_add(b)
    }
}
// ANCHOR_END: volatile

#[cfg(test)]
mod tests {
    #[test]
    fn both_read_the_same_cell() {
        let x = 21u32;
        assert_eq!(super::ch17_two_plain(&x), 42);
        // SAFETY: &x даёт валидный выровненный адрес, читаем дважды.
        assert_eq!(unsafe { super::ch17_two_volatile(&x) }, 42);
    }
}
