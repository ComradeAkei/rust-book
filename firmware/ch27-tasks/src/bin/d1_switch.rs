//! Задача 27.D1: две задачи на своих стеках и переключатель на PendSV.
//! Сначала задачи уступают друг другу сами (`yield_now`), затем задача A
//! включает SysTick и крутится, не уступая, — очередь задаче B отдаёт
//! SysTick. Порядок событий копится в журнале и печатается в конце.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-tasks --bin d1_switch
//! ```
#![no_std]
#![no_main]

use ch27_hal::sysctl::CLOCK_HZ;
use ch27_solutions::d1_switch::{RoundRobin, Stack};
use core::arch::naked_asm;
use core::sync::atomic::{AtomicU8, AtomicU32, AtomicUsize, Ordering::Relaxed};
use cortex_m::peripheral::{SCB, SYST, syst::SystClkSource};
use cortex_m::register::psp;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};

static mut STACK_A: Stack<256> = Stack::new();
static mut STACK_B: Stack<256> = Stack::new();
/// Куда PendSV сохранит r4–r11 при первом переключении: уходящей задачи
/// ещё нет, а `stmdb` по PSP выполняется всегда.
static mut BOOT: Stack<16> = Stack::new();
static mut TASKS: RoundRobin<2> = RoundRobin::new();

static SWITCHES: AtomicU32 = AtomicU32::new(0);
static SPINS: AtomicU32 = AtomicU32::new(0);
static LOG: [AtomicU8; 32] = [const { AtomicU8::new(0) }; 32];
static LEN: AtomicUsize = AtomicUsize::new(0);

fn log(bytes: &[u8]) {
    for &b in bytes {
        let i = LEN.fetch_add(1, Relaxed);
        if let Some(slot) = LOG.get(i) {
            slot.store(b, Relaxed);
        }
    }
}

// ANCHOR: pendsv
/// PendSV: r4–r11 уходящей задачи — на её стек, очередь выбирает
/// следующую, её r4–r11 — со стека в регистры. Остальное процессор
/// сохранил при входе и восстановит при выходе. EXC_RETURN 0xFFFFFFFD —
/// «в поток, на PSP»: при первом переключении PendSV пришёл из main на
/// MSP, а уходит в задачу.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn PendSV() {
    naked_asm!(
        "mrs r0, psp",
        "stmdb r0!, {{r4-r11}}",
        "bl {switch}",
        "ldmia r0!, {{r4-r11}}",
        "msr psp, r0",
        "mvn lr, #2",
        "bx lr",
        switch = sym pendsv_switch,
    );
}

extern "C" fn pendsv_switch(saved: u32) -> u32 {
    SWITCHES.fetch_add(1, Relaxed);
    // SAFETY: после старта очередь трогает только PendSV, а он не
    // вытесняет сам себя; main заполнил её до первого запроса PendSV.
    let tasks = &raw mut TASKS;
    unsafe { (*tasks).switch(saved) }
}
// ANCHOR_END: pendsv

// ANCHOR: yield
/// Уступить очередь: запросить PendSV. У него самый низкий приоритет, и
/// из потока он входит сразу после `isb`.
fn yield_now() {
    SCB::set_pendsv();
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}
// ANCHOR_END: yield

#[exception]
fn SysTick() {
    SCB::set_pendsv();
}

fn start_systick() {
    // SAFETY: SysTick трогает только задача A, один раз.
    let mut syst = unsafe { cortex_m::Peripherals::steal() }.SYST;
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(CLOCK_HZ / 1000 - 1);
    syst.clear_current();
    syst.enable_interrupt();
    syst.enable_counter();
}

extern "C" fn task_a(arg: u32) -> ! {
    log(&[arg as u8]);
    for i in 0..3 {
        log(&[b' ', b'A', b'0' + i]);
        yield_now();
    }
    start_systick();
    log(b" A*");
    loop {
        SPINS.fetch_add(1, Relaxed);
    }
}

extern "C" fn task_b(arg: u32) -> ! {
    log(&[arg as u8]);
    for i in 0..3 {
        log(&[b' ', b'B', b'0' + i]);
        yield_now();
    }
    // Сюда B попадает только по SysTick: A не уступает.
    // SAFETY: SysTick больше никому не нужен.
    unsafe { (*SYST::PTR).csr.write(0) };
    let (switches, spins) = (SWITCHES.load(Relaxed), SPINS.load(Relaxed));
    log(b" B");
    let mut buf = [0u8; 32];
    let n = LEN.load(Relaxed).min(buf.len());
    for (b, slot) in buf.iter_mut().zip(&LOG[..n]) {
        *b = slot.load(Relaxed);
    }
    hprintln!("журнал: {}", core::str::from_utf8(&buf[..n]).unwrap_or("?"));
    hprintln!("переключений {}, A крутилась без yield: {}", switches, spins > 0);
    hprintln!("PSP задачи B {:#010x}", psp::read());
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[entry]
fn main() -> ! {
    // ANCHOR: start
    // SAFETY: стеки и очередь до первого PendSV трогает только main.
    unsafe {
        let (sa, sb) = (&raw mut STACK_A, &raw mut STACK_B);
        let a = (*sa).prepare(task_a as *const () as u32, u32::from(b'['));
        let b = (*sb).prepare(task_b as *const () as u32, u32::from(b'|'));
        let tasks = &raw mut TASKS;
        (*tasks).add(a as u32).ok();
        (*tasks).add(b as u32).ok();
        let boot = &raw mut BOOT;
        psp::write((*boot).words.as_ptr_range().end as u32);
        hprintln!("начальный PSP: A {:#010x}, B {:#010x}", a as u32, b as u32);
        // PendSV — самый низкий приоритет, SysTick — выше.
        let mut cp = cortex_m::Peripherals::steal();
        cp.SCB.set_priority(cortex_m::peripheral::scb::SystemHandler::PendSV, 0xFF);
        cp.SCB.set_priority(cortex_m::peripheral::scb::SystemHandler::SysTick, 0x80);
    }
    yield_now();
    // ANCHOR_END: start
    hprintln!("main не должен продолжиться");
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
