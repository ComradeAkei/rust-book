//! Раздел 26.5: откуда в программе без libc берутся вызовы `memcpy`.
//!
//! Ни одна из функций ниже не зовёт `memcpy` явно. Вызовы вставляет
//! компилятор: копию большого объекта, заполнение и сравнение массива
//! он поручает функциям из `string.h`, а цикл, похожий на одну из них,
//! узнаёт и заменяет вызовом. На x86-64 Linux эти функции даёт libc. На
//! Cortex-M — `compiler_builtins`, и под ARM EABI вместо `memcpy` и
//! `memset` вызываются `__aeabi_memcpy`, `__aeabi_memclr4` и родня.
//!
//! Имена не искажены, чтобы найти функции в ассемблере; `inline(never)`
//! — чтобы каждая осталась отдельной функцией.

// ANCHOR: copies
/// Кадр в буфер: `copy_from_slice` — это `memcpy`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch26_copy_frame(dst: &mut [u8; 64], src: &[u8]) -> usize {
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

/// Буфер отсчётов на 64 слова. Выравнивание 4: на ARM это
/// `__aeabi_memcpy4` и `__aeabi_memclr4`, им можно копировать словами.
#[derive(Clone, Copy)]
pub struct Block {
    pub words: [u32; 64],
}

/// Присваивание 256 байт.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch26_store(slot: &mut Block, b: &Block) {
    *slot = *b;
}

/// Обнуление 256 байт.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch26_clear(slot: &mut Block) {
    slot.words = [0; 64];
}

/// Цикл заполнения: LLVM узнаёт в нём `memset`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch26_fill(buf: &mut [u8], v: u8) {
    for x in buf {
        *x = v;
    }
}

/// Сравнение массивов: `memcmp` (на x86-64 — без вызова, векторами).
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch26_same(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a == b
}

/// Длина строки C циклом: LLVM узнаёт в нём `strlen`.
///
/// # Safety
///
/// `p` указывает на байты, среди которых есть ноль.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn ch26_c_len(p: *const u8) -> usize {
    let mut n = 0;
    // SAFETY: до нулевого байта память читаема — договор вызывающего.
    while unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    n
}
// ANCHOR_END: copies

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_do_what_they_say() {
        let mut dst = [0u8; 64];
        let src: [u8; 70] = core::array::from_fn(|i| i as u8);
        assert_eq!(ch26_copy_frame(&mut dst, &src[..10]), 10);
        assert_eq!(&dst[..10], &src[..10]);
        assert_eq!(dst[10], 0);
        assert_eq!(ch26_copy_frame(&mut dst, &src), 64);
        assert_eq!(dst[..], src[..64]);

        let b = Block { words: core::array::from_fn(|i| i as u32 * 3) };
        let mut slot = Block { words: [7; 64] };
        ch26_store(&mut slot, &b);
        assert_eq!(slot.words, b.words);
        ch26_clear(&mut slot);
        assert_eq!(slot.words, [0; 64]);

        let mut buf = [1u8; 40];
        ch26_fill(&mut buf[3..30], 0xAA);
        assert!(buf[3..30].iter().all(|&x| x == 0xAA) && buf[2] == 1 && buf[30] == 1);

        let a = [5u8; 32];
        let mut c = a;
        assert!(ch26_same(&a, &c));
        c[31] = 6;
        assert!(!ch26_same(&a, &c));

        // SAFETY: строка заканчивается нулём.
        assert_eq!(unsafe { ch26_c_len(c"телеметрия".as_ptr().cast()) }, 20);
    }
}
