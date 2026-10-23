//! Глава 26: среда выполнения для Cortex-M без cortex-m-rt — таблица
//! векторов, обработчик сброса, обработчик исключений и вывод через
//! semihosting. Скрипт компоновки `ch26.ld` кладёт таблицу по адресу 0.
//!
//! Изменяемых `static` эта среда не поддерживает: её `Reset` не копирует
//! `.data` и не обнуляет `.bss`, и `ch26.ld` останавливает компоновку,
//! если такие секции появились (раздел 26.3). Полный код запуска —
//! задача 26.C.
//!
//! Библиотека только для прошивок: обработчик паники здесь, и на хосте
//! её не собирают (раздел 26.6).
#![no_std]

use core::arch::asm;

pub mod sh;

// ANCHOR: vectors
unsafe extern "C" {
    /// Конец ОЗУ: символ из `ch26.ld`. У него есть адрес, но нет объекта.
    static _stack_start: u32;
}

/// Обработчик исключения. `None` — ноль в таблице: для указателя на
/// функцию `Option` гарантированно не занимает места.
pub type Handler = Option<unsafe extern "C" fn()>;

/// Первые 16 слов таблицы векторов ARMv7-M и ARMv6-M.
#[repr(C)]
pub struct VectorTable {
    /// Слово 0: начальное значение SP. Процессор загружает его сам.
    pub initial_sp: *const u32,
    /// Слово 1: адрес обработчика сброса, с битом Thumb.
    pub reset: unsafe extern "C" fn() -> !,
    /// Слова 2–15: NMI, HardFault, MemManage, BusFault, UsageFault, четыре
    /// резервных, SVCall, DebugMonitor, резервное, PendSV, SysTick.
    pub exceptions: [Handler; 14],
}

// SAFETY: таблицу никто не меняет, а сырой указатель в ней — адрес, не
// разделяемые данные.
unsafe impl Sync for VectorTable {}

const F: Handler = Some(fault);

#[unsafe(link_section = ".vector_table")]
#[unsafe(no_mangle)]
pub static VECTORS: VectorTable = VectorTable {
    initial_sp: &raw const _stack_start,
    reset: Reset,
    exceptions: [F, F, F, F, F, None, None, None, None, F, F, None, F, F],
};
// ANCHOR_END: vectors

// ANCHOR: reset
unsafe extern "Rust" {
    /// Точка входа прошивки: её определяет макрос [`entry!`].
    fn ch26_main() -> !;
}

/// Обработчик сброса. Процессор уже загрузил SP из слова 0, и стек
/// готов. Больше ничего готовить не нужно: изменяемых static нет, а
/// `ch26.ld` это проверяет.
///
/// # Safety
///
/// Вызывает только процессор, по вектору сброса.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Reset() -> ! {
    // SAFETY: ch26_main определяет entry! с этой сигнатурой.
    unsafe { ch26_main() }
}

/// Точка входа: `entry!(main)` для `fn main() -> !`. Тип проверяет
/// присваивание, а имя `ch26_main` ищет компоновщик, — так же устроен
/// `#[entry]` из cortex-m-rt.
#[macro_export]
macro_rules! entry {
    ($main:path) => {
        #[unsafe(no_mangle)]
        fn ch26_main() -> ! {
            let f: fn() -> ! = $main;
            f()
        }
    };
}
// ANCHOR_END: reset

// ANCHOR: fault
/// Общий обработчик исключений: номер исключения — в регистре IPSR.
unsafe extern "C" fn fault() {
    let ipsr: u32;
    // SAFETY: чтение специального регистра ничего не меняет.
    unsafe { asm!("mrs {}, IPSR", out(reg) ipsr, options(nomem, nostack, preserves_flags)) };
    sh::put("исключение ");
    sh::dec(ipsr);
    sh::put("\n");
    sh::exit(false)
}
// ANCHOR_END: fault

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sh::put("паника");
    if let Some(l) = info.location() {
        sh::put(" в ");
        sh::put(l.file());
        sh::put(":");
        sh::dec(l.line());
    }
    sh::put("\n");
    sh::exit(false)
}
