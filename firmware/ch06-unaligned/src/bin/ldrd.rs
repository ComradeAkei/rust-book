//! LDRD по адресу, не кратному 4: даже Cortex-M3 падает.
#![no_std]
#![no_main]

use ch06_unaligned::core_name;
#[cfg(target_has_atomic = "32")]
use ch06_unaligned::odd_addr;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

ch06_unaligned::fault_handler!();

#[entry]
fn main() -> ! {
    #[cfg(target_has_atomic = "32")]
    {
        let aligned = odd_addr() - 1;
        let (lo, hi) = ch06_unaligned::ldrd(aligned);
        hprintln!("{}: LDRD по {:#010x}: {:#010x} {:#010x}", core_name(), aligned, lo, hi);
        hprintln!("{}: LDRD по {:#010x}", core_name(), odd_addr());
        let (lo, hi) = ch06_unaligned::ldrd(odd_addr());
        hprintln!("прочитано {:#010x} {:#010x}", lo, hi);
    }
    #[cfg(not(target_has_atomic = "32"))]
    hprintln!("{}: инструкции LDRD в ARMv6-M нет", core_name());
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
