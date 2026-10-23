//! Раздел 19.6: инструкции управления ядром на Cortex-M (`#[cfg(target_arch
//! = "arm")]`, `no_std`). Здесь интринсиков нет — к таким вещам, как
//! «уснуть до прерывания» или «запретить прерывания», доступ только через
//! `asm!`. Эти функции привязаны к ARM и проверяются сборкой под thumb
//! (`cargo build --target thumbv7em-none-eabihf`), исполнять их на хосте
//! нельзя.

use core::arch::asm;

// ANCHOR: wfi
/// `wfi` — Wait For Interrupt: остановить ядро до ближайшего прерывания.
/// Главный приём энергосбережения в цикле ожидания события.
#[inline]
pub fn wait_for_interrupt() {
    // SAFETY: wfi не трогает ни память, ни стек, ни флаги; на Cortex-M
    // доступна в thread mode. nomem честен: барьер тут не нужен — ядро просто
    // засыпает до прерывания.
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}
// ANCHOR_END: wfi

// ANCHOR: critical
/// Критическая секция: запретить прерывания, выполнить `f`, вернуть прежнее
/// состояние прерываний (а не «включить всегда» — иначе развалится
/// вложенность). `PRIMASK` — регистр-маска: бит 0 равен 1, когда прерывания
/// запрещены.
///
/// Важна правда в опциях: у `cpsid`/`cpsie` НЕТ `nomem`. Без этого компилятор
/// имел бы право переставить обращения к памяти за границу секции — и защита
/// превратилась бы в фикцию. Отсутствие `nomem` делает `asm!` барьером: всё,
/// что до него, останется до, всё, что после, — после.
pub fn with_interrupts_disabled<T>(f: impl FnOnce() -> T) -> T {
    let primask: u32;
    // SAFETY: mrs читает системный регистр PRIMASK в обычный регистр; память
    // и стек не трогает, поэтому nomem честен.
    unsafe {
        asm!("mrs {0}, PRIMASK", out(reg) primask, options(nomem, nostack, preserves_flags));
    }
    // SAFETY: cpsid i запрещает прерывания. Намеренно без nomem — это барьер.
    unsafe {
        asm!("cpsid i", options(nostack, preserves_flags));
    }

    let result = f();

    // Вернуть прежнее состояние: включить прерывания только если до секции они
    // были включены (бит 0 PRIMASK был 0).
    if primask & 1 == 0 {
        // SAFETY: cpsie i разрешает прерывания; тоже барьер — без nomem.
        unsafe {
            asm!("cpsie i", options(nostack, preserves_flags));
        }
    }
    result
}
// ANCHOR_END: critical
