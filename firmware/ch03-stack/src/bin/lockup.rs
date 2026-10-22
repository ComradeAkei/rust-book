//! Переполнение стека за нижнюю границу ОЗУ: запись в несуществующую
//! память вызывает исключение, а положить кадр исключения уже некуда.
#![no_std]
#![no_main]

use ch03_stack::dig;
use cortex_m_rt::{ExceptionFrame, entry, exception};
use cortex_m_semihosting::{debug, hprintln};

#[entry]
fn main() -> ! {
    hprintln!("рекурсия без границы");
    let levels = dig(0, 0);
    hprintln!("сюда не дойдём: {}", levels);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

// ANCHOR: hardfault
/// Обработчик есть, но вызвать его процессор не сможет: при входе в
/// исключение он кладёт восемь регистров на стек, а SP уже указывает за
/// пределы ОЗУ.
#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("HardFault, pc = {:#010x}", ef.pc());
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
// ANCHOR_END: hardfault

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
