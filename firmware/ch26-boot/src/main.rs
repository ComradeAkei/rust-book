//! Глава 26, задача C: свой код запуска для Cortex-M без cortex-m-rt.
//!
//! Таблица векторов на Rust, обработчик сброса на ассемблере: обнулить
//! `.bss`, скопировать `.data` из flash, открыть FPU на Cortex-M4F и
//! вызвать `main`. Проверка — три загрузки подряд через программный
//! сброс: счётчик в `.uninit` растёт, а `.data` и `.bss` каждый раз
//! начинаются заново.
//!
//! ```text
//! cd firmware && cargo run --release -p ch26-boot
//! ```
#![no_std]
#![no_main]

use core::arch::{asm, naked_asm};
use core::mem::MaybeUninit;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: vectors
unsafe extern "C" {
    static _stack_start: u32;
}

type Handler = Option<unsafe extern "C" fn()>;

#[repr(C)]
pub struct VectorTable {
    initial_sp: *const u32,
    reset: unsafe extern "C" fn() -> !,
    exceptions: [Handler; 14],
}

// SAFETY: таблицу никто не меняет.
unsafe impl Sync for VectorTable {}

const D: Handler = Some(default_handler);

#[unsafe(link_section = ".vector_table")]
#[unsafe(no_mangle)]
pub static VECTORS: VectorTable = VectorTable {
    initial_sp: &raw const _stack_start,
    reset: Reset,
    // NMI, HardFault, MemManage, BusFault, UsageFault, 4 резервных, SVCall,
    // DebugMonitor, резервное, PendSV, SysTick.
    exceptions: [D, D, D, D, D, None, None, None, None, D, D, None, D, D],
};
// ANCHOR_END: vectors

// ANCHOR: fpu
// Cortex-M4F: до первой инструкции FPU сопроцессоры CP10 и CP11 надо
// открыть в CPACR, иначе UsageFault NOCP (задача 0.D). Компилятор под
// thumbv7em-none-eabihf вправе пользоваться регистрами FPU где угодно.
#[cfg(target_abi = "eabihf")]
macro_rules! fpu_on {
    () => {
        "ldr r0, =0xE000ED88
         ldr r1, [r0]
         orr r1, r1, #0xF00000
         str r1, [r0]
         dsb
         isb"
    };
}
#[cfg(not(target_abi = "eabihf"))]
macro_rules! fpu_on {
    () => {
        ""
    };
}
// ANCHOR_END: fpu

// ANCHOR: reset
/// Обработчик сброса. До конца двух циклов ни одна строка Rust не
/// выполняется: компилятор вправе считать, что каждый `static` уже
/// содержит своё начальное значение. Только инструкции ARMv6-M — один
/// код на Cortex-M0, M3 и M4F.
///
/// # Safety
///
/// Вызывает только процессор, по вектору сброса.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Reset() -> ! {
    naked_asm!(
        // .bss: нули словами от __sbss до __ebss.
        "ldr r0, =__sbss",
        "ldr r1, =__ebss",
        "movs r2, #0",
        "1:",
        "cmp r0, r1",
        "bhs 2f",
        "stm r0!, {{r2}}",
        "b 1b",
        // .data: слова из flash (__sidata) в ОЗУ (__sdata..__edata).
        "2:",
        "ldr r0, =__sdata",
        "ldr r1, =__edata",
        "ldr r2, =__sidata",
        "3:",
        "cmp r0, r1",
        "bhs 4f",
        "ldm r2!, {{r3}}",
        "stm r0!, {{r3}}",
        "b 3b",
        "4:",
        fpu_on!(),
        "bl {main}",
        "udf #0",
        // Литералы для ldr r0, =… — здесь, рядом с функцией.
        ".ltorg",
        main = sym main,
    )
}
// ANCHOR_END: reset

// ANCHOR: handler
/// Все исключения: номер из IPSR и выход с ошибкой.
unsafe extern "C" fn default_handler() {
    let ipsr: u32;
    // SAFETY: чтение специального регистра ничего не меняет.
    unsafe { asm!("mrs {}, IPSR", out(reg) ipsr, options(nomem, nostack, preserves_flags)) };
    hprintln!("исключение {}", ipsr);
    debug::exit(debug::EXIT_FAILURE);
}
// ANCHOR_END: handler

// ANCHOR: statics
/// `.data`: начальное значение 5 копирует код запуска.
static mut DATA: u32 = 5;
/// `.bss`: ноль даёт код запуска.
static mut BSS: u32 = 0;
/// `.uninit`: метка и номер загрузки, переживают программный сброс.
#[unsafe(link_section = ".uninit")]
static mut BOOT: MaybeUninit<[u32; 2]> = MaybeUninit::uninit();
const MAGIC: u32 = 0x2600_B007;
// ANCHOR_END: statics

// ANCHOR: main
extern "C" fn main() -> ! {
    // SAFETY: прерываний нет, к статическим переменным обращается только
    // этот код. Любые 8 байт — допустимое значение [u32; 2]; volatile — как
    // в главе 8: компилятор не знает, что лежит в .uninit, и не выбросит
    // запись перед сбросом.
    let (boot, data, bss) = unsafe {
        let b = &raw mut BOOT as *mut [u32; 2];
        let [magic, n] = b.read_volatile();
        let n = if magic == MAGIC { n + 1 } else { 0 };
        b.write_volatile([MAGIC, n]);
        DATA += 1;
        BSS += 1;
        (n, (&raw const DATA).read_volatile(), (&raw const BSS).read_volatile())
    };
    hprintln!("загрузка {}: DATA = {} (ожидалось 6), BSS = {} (ожидалось 1)", boot, data, bss);
    if data != 6 || bss != 1 {
        debug::exit(debug::EXIT_FAILURE);
    }
    if boot < 2 {
        // SCB->AIRCR: ключ 0x05FA и SYSRESETREQ — программный сброс.
        // SAFETY: запись в регистр ядра; после неё процессор сбрасывается.
        unsafe {
            asm!("dsb", options(nostack, preserves_flags));
            (0xE000_ED0C as *mut u32).write_volatile(0x05FA_0004);
            asm!("dsb", options(nostack, preserves_flags));
        }
        loop {}
    }
    // Cortex-M4F: умножение и преобразование в целое — инструкции FPU.
    // Без fpu_on!() в Reset здесь был бы UsageFault NOCP.
    #[cfg(target_abi = "eabihf")]
    {
        let x = core::hint::black_box(1.5f32) * core::hint::black_box(3.0f32);
        hprintln!("FPU открыт: 1.5 * 3.0 * 2 = {}", (x * 2.0) as u32);
    }
    hprintln!("три загрузки: .data и .bss каждый раз заново, .uninit сохранился");
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    hprintln!("паника: {}", info);
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
