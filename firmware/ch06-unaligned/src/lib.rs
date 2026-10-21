//! Глава 6, раздел «Железо»: что делает процессор с невыровненным адресом.
//!
//! Каждый опыт — отдельная прошивка в `src/bin`. Инструкции записаны
//! встроенным ассемблером: в Rust невыровненное разыменование указателя —
//! UB, и компилятор вправе превратить его во что угодно. Нам же нужна
//! ровно одна конкретная инструкция.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch06-unaligned --bin ldr                               # Cortex-M3
//! cargo run --release -p ch06-unaligned --bin ldr --target thumbv6m-none-eabi   # Cortex-M0
//! cargo run --release -p ch06-unaligned --bin ldrd
//! cargo run --release -p ch06-unaligned --bin trap
//! cargo run --release -p ch06-unaligned --bin trap_fmt
//! ```
#![no_std]

pub use cortex_m_rt;
pub use cortex_m_semihosting;

/// Восемь байт с выравниванием 8. Слово по адресу `BYTES + 1` в порядке
/// little-endian равно 0x4433_2211.
#[repr(C, align(8))]
pub struct Bytes(pub [u8; 16]);

pub static BYTES: Bytes = Bytes([
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
]);

pub fn odd_addr() -> usize {
    BYTES.0.as_ptr() as usize + 1
}

// ANCHOR: ldr
/// Одна инструкция LDR по произвольному адресу.
#[inline(never)]
pub fn ldr(addr: usize) -> u32 {
    let v: u32;
    // SAFETY: адрес указывает внутрь BYTES; ловушку при невыровненном
    // адресе мы и хотим увидеть, а памяти инструкция не меняет.
    unsafe {
        core::arch::asm!("ldr {v}, [{a}]", v = lateout(reg) v, a = in(reg) addr,
                         options(nostack, readonly, preserves_flags));
    }
    v
}
// ANCHOR_END: ldr

// ANCHOR: ldrd
/// LDRD читает два слова одной инструкцией. В ARMv6-M её нет.
#[cfg(target_has_atomic = "32")]
#[inline(never)]
pub fn ldrd(addr: usize) -> (u32, u32) {
    let (lo, hi): (u32, u32);
    // SAFETY: как у `ldr`; адрес + 8 не выходит за BYTES.
    unsafe {
        core::arch::asm!("ldrd {lo}, {hi}, [{a}]", lo = lateout(reg) lo, hi = lateout(reg) hi,
                         a = in(reg) addr, options(nostack, readonly, preserves_flags));
    }
    (lo, hi)
}
// ANCHOR_END: ldrd

// ANCHOR: trap
/// Регистр SCB->CCR, бит 3 UNALIGN_TRP: ловить любой невыровненный доступ.
/// В ARMv6-M этот бит всегда читается как 1.
pub const SCB_CCR: *mut u32 = 0xE000_ED14 as *mut u32;
pub const CCR_UNALIGN_TRP: u32 = 1 << 3;

pub fn enable_unaligned_trap() {
    // SAFETY: CCR существует на любом Cortex-M, запись бита 3 разрешена в v7-M.
    unsafe { SCB_CCR.write_volatile(SCB_CCR.read_volatile() | CCR_UNALIGN_TRP) }
}

pub fn disable_unaligned_trap() {
    // SAFETY: см. `enable_unaligned_trap`.
    unsafe { SCB_CCR.write_volatile(SCB_CCR.read_volatile() & !CCR_UNALIGN_TRP) }
}
// ANCHOR_END: trap

/// Имя ядра для отчёта.
pub const fn core_name() -> &'static str {
    if cfg!(target_has_atomic = "32") { "Cortex-M3 (ARMv7-M)" } else { "Cortex-M0 (ARMv6-M)" }
}

// ANCHOR: fault
/// Обработчик HardFault печатает, где и почему упал процессор, и завершает
/// QEMU. Регистры CFSR и HFSR есть только в ARMv7-M; в ARMv6-M любая
/// ошибка сразу становится HardFault без уточняющего статуса.
#[macro_export]
macro_rules! fault_handler {
    () => {
        #[$crate::cortex_m_rt::exception]
        unsafe fn HardFault(ef: &$crate::cortex_m_rt::ExceptionFrame) -> ! {
            use $crate::cortex_m_semihosting::{debug, hprintln};
            // Сначала снимаем ловушку: код форматирования из core сам
            // пользуется невыровненными LDR/STR, и без этого печать внутри
            // обработчика упадёт повторно, а QEMU сообщит Lockup.
            #[cfg(target_has_atomic = "32")]
            $crate::disable_unaligned_trap();
            hprintln!("HardFault на {}: PC = {:#010x}", $crate::core_name(), ef.pc());
            #[cfg(target_has_atomic = "32")]
            {
                // SAFETY: адреса регистров SCB из ARMv7-M Architecture Reference Manual.
                let cfsr = unsafe { (0xE000_ED28 as *const u32).read_volatile() };
                let hfsr = unsafe { (0xE000_ED2C as *const u32).read_volatile() };
                hprintln!("CFSR = {:#010x}, HFSR = {:#010x}", cfsr, hfsr);
                if cfsr & (1 << 24) != 0 {
                    hprintln!("UFSR.UNALIGNED: невыровненный доступ");
                }
                if hfsr & (1 << 30) != 0 {
                    hprintln!("HFSR.FORCED: UsageFault выключен и повышен до HardFault");
                }
            }
            debug::exit(debug::EXIT_FAILURE);
            loop {}
        }
    };
}
// ANCHOR_END: fault

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_FAILURE);
    loop {}
}
