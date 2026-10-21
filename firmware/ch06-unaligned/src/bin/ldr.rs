//! LDR по нечётному адресу: Cortex-M3 читает слово, Cortex-M0 падает.
#![no_std]
#![no_main]

use ch06_unaligned::{core_name, ldr, odd_addr};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

ch06_unaligned::fault_handler!();

#[entry]
fn main() -> ! {
    let addr = odd_addr();
    hprintln!("{}: LDR по адресу {:#010x}", core_name(), addr);
    let v = ldr(addr);
    hprintln!("прочитано {:#010x}", v);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
