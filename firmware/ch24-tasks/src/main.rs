//! Глава 24, задачи B–D, на Cortex-M3 (lm3s6965evb). Блок регистров UART0
//! порождён макросом `include_svd!` из файла SVD (задача D), формат кадра в
//! LCRH собран типом `LcrhBits` из `#[derive(Bitfield)]` (задача B), а
//! функция передачи замерена `#[timed]` по часам SysTick (задача C).
//! Строки уходят в UART0 — в QEMU это стандартный вывод, — остальное
//! печатается через semihosting.
//!
//! ```text
//! cd firmware && cargo run --release -p ch24-tasks
//! ```
//!
//! Собирается под все три цели, но UART0 по адресу 0x4000_C000 есть только
//! у lm3s6965evb — запускается прошивка на M3. На ARMv6-M нет `fetch_add`,
//! а значит, и `Timing`: там замер выключен через `cfg_attr`.
#![no_std]
#![no_main]

use ch24_solutions::b_bitfield::{LCRH_8N1_FIFO, Lcrh, LcrhBits};
use ch24_solutions::c_timed::SysTickClock;
use ch24_solutions::include_svd;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: svd
// Путь — от каталога этого крейта, firmware/ch24-tasks.
include_svd!("../../code/ch24/svd/lm3s6965.svd", "UART0");
// ANCHOR_END: svd

// ANCHOR: timed
#[cfg(target_has_atomic = "32")]
static SEND: ch24_solutions::c_timed::Timing<SysTickClock> = ch24_solutions::c_timed::Timing::new();

/// Строка в UART0: байт за байтом, пока в FIFO передатчика есть место.
#[cfg_attr(target_has_atomic = "32", ch24_solutions::timed(SEND))]
fn send(uart: &uart0::RegisterBlock, s: &str) {
    for b in s.bytes() {
        while uart0::Fr::TXFF.get(uart.fr.read()) != 0 {}
        uart.dr.write(u32::from(b));
    }
}
// ANCHOR_END: timed

// ANCHOR: main
#[entry]
fn main() -> ! {
    // SAFETY: прошивка — единственный владелец UART0 на этой машине.
    let uart = unsafe { uart0::steal() };

    // Значения после сброса из файла SVD против того, что читает устройство.
    hprintln!("CTL: 0x{:03x}, по SVD 0x{:03x}", uart.ctl.read(), uart0::Ctl::RESET);
    hprintln!("IFLS: 0x{:02x}, по SVD 0x{:02x}", uart.ifls.read(), uart0::Ifls::RESET);

    // Формат кадра — константой из задачи B, обратно — её же разбором и
    // полем WLEN из SVD.
    uart.lcrh.write(LCRH_8N1_FIFO.0);
    let lcrh = uart.lcrh.read();
    hprintln!("LCRH = {:?}", Lcrh::from(LcrhBits(lcrh)));
    hprintln!("LCRH.WLEN по SVD: {}", uart0::Lcrh::WLEN.get(lcrh));

    uart.ctl.modify(|v| v | uart0::Ctl::UARTEN.mask() | uart0::Ctl::TXE.mask());
    // SAFETY: SysTick в этой прошивке больше никто не настраивает.
    unsafe { SysTickClock::start() };
    for line in
        ["uart0: hello from include_svd!\n", "uart0: SVD -> RegisterBlock\n", "uart0: #[timed]\n"]
    {
        send(uart, line);
    }

    #[cfg(target_has_atomic = "32")]
    {
        let (calls, total, max) = (SEND.calls(), SEND.total(), SEND.max());
        // hprintln! склеивает строку формата через concat!, и имена в {}
        // он не захватывает: только позиционные аргументы.
        hprintln!("send: вызовов {}, самый долгий не дольше всех: {}", calls, max <= total);
        // Сколько тактов — зависит от хоста: QEMU ведёт SysTick по своим часам.
        hprintln!("send: {} тактов SysTick, самый долгий {}", total, max);
        // SAFETY: DWT в этой прошивке больше никто не настраивает.
        let dwt = unsafe { ch24_solutions::c_timed::DwtCycles::enable() };
        hprintln!("DWT.CYCCNT включился: {}", dwt);
    }
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
