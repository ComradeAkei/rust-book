//! Глава 27, раздел 27.7: сторож стека на MPU. Рабочий код живёт на PSP
//! в своём стеке на 4 КиБ, нижние 256 байт стека MPU закрывает для
//! любого доступа. Рекурсия без границы доходит до сторожа, и первая же
//! запись в него — MemManage. Обработчик работает на MSP, своём стеке,
//! поэтому может спокойно напечатать отчёт.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-fault --bin guard
//! ```
#![no_std]
#![no_main]

use ch27::mpu::{Access, rasr, rbar};
use core::arch::naked_asm;
use core::hint::black_box;
use cortex_m::peripheral::{MPU, SCB};
use cortex_m::register::{msp, psp};
use cortex_m_rt::{ExceptionFrame, entry, exception};
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: stack
/// Размер сторожа — степень двойки: так требует MPU. 256 байт больше
/// кадра любой функции здесь, и SP не перепрыгнет сторожа.
const GUARD_LOG2: u8 = 8;
const STACK_SIZE: usize = 4096;

/// Стек рабочего кода. Выравнивание — на размер сторожа: начало региона
/// MPU выровнено на его размер.
#[repr(C, align(256))]
struct Stack([u8; STACK_SIZE]);

static mut TASK_STACK: Stack = Stack([0; STACK_SIZE]);
// ANCHOR_END: stack

/// Рекурсия «без границы»: условие выхода есть, но не срабатывает
/// никогда — `black_box` не даёт компилятору это увидеть.
#[inline(never)]
fn dig(level: u32) -> u32 {
    let mut buf = [0xEE_u8; 128];
    black_box(&mut buf);
    if black_box(level) == u32::MAX {
        return level;
    }
    let deeper = dig(level + 1);
    black_box(&buf);
    deeper
}

/// Перейти на PSP с вершиной `top` и продолжить в `next` (раздел 27.1).
#[unsafe(naked)]
unsafe extern "C" fn switch_to_psp(top: usize, next: extern "C" fn() -> !) -> ! {
    naked_asm!(
        "msr psp, r0",
        "mrs r2, control",
        "orr r2, r2, #2",
        "msr control, r2",
        "isb",
        "bx r1"
    );
}

extern "C" fn task() -> ! {
    hprintln!("рекурсия без границы на PSP");
    black_box(dig(0));
    hprintln!("сюда не дойдём");
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

#[entry]
fn main() -> ! {
    let bottom = (&raw const TASK_STACK) as usize;
    hprintln!(
        "стек задачи {:#010x}..{:#010x}, сторож {:#010x}..{:#010x}",
        bottom,
        bottom + STACK_SIZE,
        bottom,
        bottom + (1 << GUARD_LOG2)
    );
    // ANCHOR: mpu
    // SAFETY: регион 0 закрывает только дно стека задачи; остальная
    // память доступна по карте по умолчанию (PRIVDEFENA).
    unsafe {
        let mpu = &*MPU::PTR;
        mpu.rbar.write(rbar(bottom as u32, GUARD_LOG2, 0));
        mpu.rasr.write(rasr(GUARD_LOG2, Access::None, true));
        mpu.ctrl.write(1 << 2 | 1); // PRIVDEFENA | ENABLE
        cortex_m::asm::dsb();
        cortex_m::asm::isb();
        // Без MEMFAULTENA нарушение MPU сразу стало бы HardFault.
        (*SCB::PTR).shcsr.modify(|v| v | 1 << 16);
        switch_to_psp(bottom + STACK_SIZE, task)
    }
    // ANCHOR_END: mpu
}

// ANCHOR: memmanage
#[exception]
fn MemoryManagement() {
    // SAFETY: регистры состояния сбоя только читаются.
    let (cfsr, mmfar) = unsafe { ((*SCB::PTR).cfsr.read(), (*SCB::PTR).mmfar.read()) };
    hprintln!("MemManage: CFSR {:#010x}, MMFAR {:#010x}", cfsr, mmfar);
    hprintln!("PSP {:#010x}, обработчик на MSP {:#010x}", psp::read(), msp::read());
    debug::exit(debug::EXIT_SUCCESS);
}
// ANCHOR_END: memmanage

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("HardFault: pc {:#010x}", ef.pc());
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
