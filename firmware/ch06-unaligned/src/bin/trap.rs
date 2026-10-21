//! Тот же LDR, что в `ldr.rs`, но с включённым CCR.UNALIGN_TRP.
#![no_std]
#![no_main]

use ch06_unaligned::{CCR_UNALIGN_TRP, SCB_CCR, core_name, enable_unaligned_trap, ldr, odd_addr};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

ch06_unaligned::fault_handler!();

#[entry]
fn main() -> ! {
    // Печатаем до включения ловушки: см. `trap_fmt.rs`, почему не после.
    // SAFETY: чтение CCR не имеет побочных эффектов.
    let ccr = unsafe { SCB_CCR.read_volatile() };
    hprintln!("{}: CCR = {:#010x}, UNALIGN_TRP = {}", core_name(), ccr, ccr & CCR_UNALIGN_TRP != 0);
    hprintln!("включаем UNALIGN_TRP и повторяем LDR по {:#010x}", odd_addr());
    enable_unaligned_trap();
    let v = ldr(odd_addr());
    hprintln!("прочитано {:#010x}", v);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
