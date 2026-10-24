//! Переполнение стека за нижнюю границу ОЗУ. На кристалле запись в
//! несуществующую память — ошибка шины. QEMU такие записи отбрасывает, а
//! чтения дают ноль: первым ломается возврат по адресу со стека — переход
//! на 0, INVSTATE и HardFault. Стека нет и у обработчика, и его
//! собственный сбой — lockup (раздел 3.5).
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
/// Обработчик есть, и процессор в него войдёт, хотя восемь регистров
/// кадра положить некуда (BusFault STKERR). Но стека нет и у обработчика:
/// `ef` и параметры semihosting читаются нулями, первый возврат уходит на
/// адрес 0, а сбой при выполнении самого HardFault — lockup.
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
