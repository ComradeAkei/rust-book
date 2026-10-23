//! Глава 22, задача D1, на Cortex-M3 (lm3s6965evb): две задачи без кучи.
//! Одну будит SysTick, другую — приём байта по UART0; исполнитель опрашивает
//! только разбуженную, а пока будить некому, ядро спит в `wfi`. Задачи
//! лежат в арене из `'static`-буфера.
//!
//! Строку для UART0 QEMU берёт со стандартного ввода:
//!
//! ```text
//! cd firmware && printf 'ping\n' | cargo run --release -p ch22-tasks
//! ```
//!
//! Собирается под все три цели, но UART0 (PL011 по адресу 0x4000_C000,
//! прерывание 5) есть только у lm3s6965evb — запускается прошивка на M3.
#![no_std]
#![no_main]

use ch22_solutions::d1_static::{self, Arena, ByteQueue, Executor, Signal};
use core::mem::MaybeUninit;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::AtomicBool;
use cortex_m::interrupt::InterruptNumber;
use cortex_m::peripheral::NVIC;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: statics
/// Флаги готовности задач и источники событий: всё статическое, без кучи.
static FLAGS: [AtomicBool; 2] = [const { AtomicBool::new(false) }; 2];
static TICKS: Signal = Signal::new(&FLAGS[0]); // будит задачу 0
static RX: ByteQueue<16> = ByteQueue::new(&FLAGS[1]); // будит задачу 1
// ANCHOR_END: statics

const RELOAD: u32 = 1_000_000;
const ARENA: usize = 256;

/// UART0 — PL011 (регистры описаны в главе 6). Здесь нужны ещё маска
/// прерываний IMSC и номер прерывания в NVIC.
const UART0: usize = 0x4000_C000;
const DR: usize = 0x000;
const FR: usize = 0x018;
const CR: usize = 0x030;
const IMSC: usize = 0x038;
const FR_RXFE: u32 = 1 << 4; // FIFO приёма пуст
const IM_RX: u32 = 1 << 4; // прерывание по приёму
const CR_EN: u32 = 0x301; // UARTEN | TXE | RXE

fn uart(offset: usize) -> *mut u32 {
    (UART0 + offset) as *mut u32
}

/// Номер внешнего прерывания UART0 на lm3s6965evb. Без крейта устройства
/// (PAC) номер задаём сами.
#[derive(Clone, Copy)]
struct Uart0Irq;

// SAFETY: 5 — действительный номер прерывания UART0 этой машины.
unsafe impl InterruptNumber for Uart0Irq {
    fn number(self) -> u16 {
        5
    }
}

// ANCHOR: handlers
#[exception]
fn SysTick() {
    TICKS.raise(); // тик — задаче 0
}

/// Без PAC cortex-m-rt отправляет все внешние прерывания сюда, с номером.
#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    if irqn == Uart0Irq.number() as i16 {
        // SAFETY: регистры UART0; чтение DR, пока FIFO не пуст, снимает
        // и запрос прерывания.
        unsafe {
            while read_volatile(uart(FR)) & FR_RXFE == 0 {
                RX.push(read_volatile(uart(DR)) as u8); // байт — задаче 1
            }
        }
    }
}
// ANCHOR_END: handlers

// ANCHOR: tasks
async fn ticker(n: u32) {
    let mut seen = 0;
    for _ in 0..n {
        let t = TICKS.wait(&mut seen).await;
        hprintln!("тик {}", t);
    }
}

async fn line() {
    let mut buf = [0u8; 16];
    let mut len = 0;
    while len < buf.len() {
        match RX.pop().await {
            b'\n' => break,
            b => buf[len] = b,
        }
        len += 1;
    }
    hprintln!("uart: «{}»", core::str::from_utf8(&buf[..len]).unwrap_or("?"));
}

#[entry]
fn main() -> ! {
    let mut p = cortex_m::Peripherals::take().unwrap();
    // SAFETY: UART0 и его прерывание трогает только эта прошивка.
    unsafe {
        write_volatile(uart(CR), CR_EN);
        write_volatile(uart(IMSC), IM_RX);
        NVIC::unmask(Uart0Irq);
    }
    p.SYST.set_clock_source(SystClkSource::Core);
    p.SYST.set_reload(RELOAD);
    p.SYST.clear_current();
    p.SYST.enable_interrupt();
    p.SYST.enable_counter();

    // Буфер арены: singleton! выдаёт &'static mut ровно один раз.
    let buf =
        cortex_m::singleton!(: [MaybeUninit<u8>; ARENA] = [MaybeUninit::uninit(); ARENA]).unwrap();
    let mut arena = Arena::new(buf);
    let tasks = [arena.alloc(ticker(3)).unwrap(), arena.alloc(line()).unwrap()];
    hprintln!("арена: задачи заняли {} из {} байт", ARENA - arena.remaining(), ARENA);

    let report = Executor::new(&FLAGS, tasks).run(d1_static::cortex_idle);
    p.SYST.disable_counter();
    hprintln!(
        "задачи завершены: опросов {:?}, снов в wfi {}, потеряно байт {}",
        report.polls,
        report.idles,
        RX.lost()
    );
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: tasks

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
