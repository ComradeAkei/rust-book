//! Ловушка на невыровненный доступ ломает часть обычной печати: машинный
//! код core::fmt для thumbv7m местами сам читает и пишет слова по
//! невыровненным адресам, потому что для ARMv7-M это разрешено.
#![no_std]
#![no_main]

use ch06_unaligned::{core_name, enable_unaligned_trap};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

ch06_unaligned::fault_handler!();

#[entry]
fn main() -> ! {
    hprintln!("{}: включаем UNALIGN_TRP и печатаем число", core_name());
    enable_unaligned_trap();
    hprintln!("десятичное: {}", 1234567u32);
    hprintln!("шестнадцатеричное: {:#010x}", 1234567u32);
    hprintln!("печать пережила ловушку");
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
