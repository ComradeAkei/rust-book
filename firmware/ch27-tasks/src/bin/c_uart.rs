//! Задача 27.C: консоль станции на UART0. Обработчик прерывания приёма
//! перекладывает байты в SPSC-очередь главы 21, main собирает из них
//! строки редактором с эхом и выполняет команды. Строку `quit` прошивка
//! понимает сама, до разбора: это выход из QEMU для опыта, а не команда
//! станции.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-tasks --bin c_uart
//! ```
//!
//! QEMU берёт ввод для UART0 со стандартного ввода; Enter — это `\r`.
#![no_std]
#![no_main]

use ch21::spsc::Spsc;
use ch27_hal::pac::{self, Interrupt, interrupt};
use ch27_hal::prelude::*;
use ch27_hal::serial::{self, Event, Serial};
use ch27_hal::sysctl::Sysctl;
use ch27_solutions::c_lines::{Command, Feed, LineEditor, Sink, parse};
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use cortex_m_semihosting::debug;

// ANCHOR: rx
/// Очередь приёма. На линии 115200 бод байты идут раз в 87 мкс, и main
/// успевает разбирать их по одному. В QEMU весь ввод приходит без пауз,
/// поэтому очередь вмещает весь ввод опыта. Писатель — обработчик UART0,
/// читатель — `next_byte` в main.
static RX: Spsc<u8, 128> = Spsc::new();
static LOST: AtomicU32 = AtomicU32::new(0);

#[interrupt]
fn UART0() {
    // SAFETY: обработчик только читает FR и DR и сбрасывает запрос
    // тайм-аута в ICR; main через `Serial` только пишет DR и читает FR.
    // Регистры у них общие, но поля — разные.
    let uart = unsafe { &*pac::Uart0::ptr() };
    while uart.fr().read().rxfe().bit_is_clear() {
        // SAFETY: писатель RX — только этот обработчик, сам себя он не
        // вытесняет.
        if unsafe { RX.push(uart.dr().read().data().bits()) }.is_err() {
            LOST.fetch_add(1, Relaxed);
        }
    }
    // Запрос RX снимает само чтение FIFO, запрос тайм-аута — только ICR.
    // RX сбрасываем и в ICR: если байт пришёл до настройки UART, QEMU
    // держит запрос RX и при пустом FIFO, и без сброса обработчик входил
    // бы снова и снова (раздел 27.6).
    uart.icr().write(|w| w.rxic().set_bit().rtic().set_bit());
}
// ANCHOR_END: rx

// ANCHOR: sleep
/// Следующий байт из очереди; пока её нет — сон. Проверка и `wfi` — при
/// запрещённых прерываниях: иначе байт, пришедший между пустой проверкой
/// и `wfi`, пролежал бы в очереди, а процессор уснул бы до следующего
/// байта. Запрос, ждущий при PRIMASK = 1, всё равно будит `wfi`, а
/// обработчик выполняется сразу после `cpsie i`.
fn next_byte() -> u8 {
    loop {
        cortex_m::interrupt::disable();
        // SAFETY: читатель RX — только main.
        let byte = unsafe { RX.pop() };
        if byte.is_none() {
            cortex_m::asm::wfi();
        }
        // SAFETY: main не держит критических секций, прерывания были
        // разрешены до `disable`.
        unsafe { cortex_m::interrupt::enable() };
        if let Some(b) = byte {
            return b;
        }
    }
}
// ANCHOR_END: sleep

/// Эхо редактора — в тот же UART.
struct Echo<'a>(&'a mut Serial);

impl Sink for Echo<'_> {
    fn put(&mut self, bytes: &[u8]) {
        let Ok(()) = self.0.write_all(bytes);
    }
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut sc = Sysctl::new(dp.sysctl);
    let pa = dp.gpioa.split(&mut sc);
    let pins = (pa.pa0.into_alternate(), pa.pa1.into_alternate());
    let mut uart = Serial::new(dp.uart0, pins, serial::baud(115_200), &mut sc);
    uart.listen(Event::Rx);
    // SAFETY: обработчик UART0 готов, очередь пуста.
    unsafe { NVIC::unmask(Interrupt::UART0) };
    writeln!(uart, "команды: status, start, stop, period <мс>").ok();

    // ANCHOR: console
    let mut editor = LineEditor::<32>::new();
    let (mut period, mut running, mut lines) = (100, false, 0u32);
    loop {
        let byte = next_byte();
        let line = match editor.feed(byte, &mut Echo(&mut uart)) {
            Feed::Pending => continue,
            Feed::TooLong => {
                writeln!(uart, "ошибка: строка длиннее 32 байт").ok();
                continue;
            }
            Feed::Line(line) => line,
        };
        lines += 1;
        if line == "quit" {
            break;
        }
        match parse(line) {
            Ok(Command::Status) => {
                let state = if running { "идёт" } else { "остановлена" };
                writeln!(uart, "период {} мс, {}", period, state).ok()
            }
            Ok(Command::Start) => {
                running = true;
                writeln!(uart, "ok: запущена").ok()
            }
            Ok(Command::Stop) => {
                running = false;
                writeln!(uart, "ok: остановлена").ok()
            }
            Ok(Command::Period(ms)) => {
                period = ms;
                writeln!(uart, "ok: период {} мс", ms).ok()
            }
            Err(e) => writeln!(uart, "ошибка: {}", e.message()).ok(),
        };
    }
    // ANCHOR_END: console
    writeln!(uart, "строк {}, потеряно байт {}", lines, LOST.load(Relaxed)).ok();
    let Ok(()) = uart.flush();
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
