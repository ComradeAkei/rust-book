//! Раздел 8.3: одна функция, две реализации — выбор при компиляции по
//! признаку цели `target_has_atomic`.
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::Relaxed;

// ANCHOR: bump
/// Увеличить счётчик событий и вернуть прежнее значение. Безопасно и из
/// обработчика прерывания.
#[cfg(target_has_atomic = "32")]
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch08_bump(c: &AtomicU32) -> u32 {
    c.fetch_add(1, Relaxed)
}

/// ARMv6-M (Cortex-M0): атомарных чтения-изменения-записи нет, только
/// загрузка и сохранение. Запрещаем прерывания на время двух операций.
#[cfg(all(not(target_has_atomic = "32"), target_arch = "arm"))]
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch08_bump(c: &AtomicU32) -> u32 {
    let primask: u32;
    // SAFETY: читаем PRIMASK и запрещаем прерывания; стек и флаги не
    // трогаем. Без `nomem` вставка служит барьером для компилятора: он не
    // перенесёт load и store за её пределы.
    unsafe {
        core::arch::asm!("mrs {}, PRIMASK", "cpsid i", out(reg) primask,
            options(nostack, preserves_flags))
    };
    let v = c.load(Relaxed);
    c.store(v.wrapping_add(1), Relaxed);
    if primask & 1 == 0 {
        // SAFETY: прерывания были разрешены до нас — возвращаем как было.
        unsafe { core::arch::asm!("cpsie i", options(nostack, preserves_flags)) };
    }
    v
}

#[cfg(not(any(target_has_atomic = "32", target_arch = "arm")))]
compile_error!("ch08_bump: на этой цели нет ни атомиков, ни способа запретить прерывания");
// ANCHOR_END: bump

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump() {
        let c = AtomicU32::new(u32::MAX - 1);
        assert_eq!(ch08_bump(&c), u32::MAX - 1);
        assert_eq!(ch08_bump(&c), u32::MAX);
        assert_eq!(c.load(Relaxed), 0);
    }
}
