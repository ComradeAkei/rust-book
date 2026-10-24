//! Глава 27, разделы 27.3–27.5: от PAC к HAL. Прошивка берёт периферию
//! через `Peripherals::take()`, делит порты на выводы, печатает через
//! UART0 (в QEMU он выходит в терминал) и выдвигает байт в 74HC595 на
//! выводах PB0–PB2 драйвером из `ch27::hc595`.
//!
//! ```text
//! cd firmware && cargo run --release -p ch27-periph
//! ```
#![no_std]
#![no_main]

use ch27::hc595::Hc595;
use ch27_hal::delay::Delay;
use ch27_hal::gpio::{Output, PF0};
use ch27_hal::pac;
use ch27_hal::prelude::*;
use ch27_hal::serial::{self, Serial};
use ch27_hal::sysctl::Sysctl;
use cortex_m_rt::entry;
use cortex_m_semihosting::debug;

#[entry]
fn main() -> ! {
    // ANCHOR: take
    let dp = pac::Peripherals::take().unwrap();
    let again = pac::Peripherals::take().is_none();
    let cp = pac::CorePeripherals::take().unwrap();
    // ANCHOR_END: take

    // ANCHOR: serial
    let mut sc = Sysctl::new(dp.sysctl);
    let pa = dp.gpioa.split(&mut sc);
    let pins = (pa.pa0.into_alternate(), pa.pa1.into_alternate());
    const BAUD: ch25::const_fn::Divisor = serial::baud(115_200);
    let mut uart = Serial::new(dp.uart0, pins, BAUD, &mut sc);
    // ANCHOR_END: serial
    writeln!(uart, "UART0: 115200 бод при 12 МГц, IBRD {}, FBRD {}", BAUD.ibrd, BAUD.fbrd).ok();
    writeln!(uart, "Peripherals::take() второй раз: {}", if again { "None" } else { "Some" }).ok();
    writeln!(
        uart,
        "размер Peripherals {} байт, PF0<Output> {} байт",
        size_of::<pac::Peripherals>(),
        size_of::<PF0<Output>>()
    )
    .ok();

    // ANCHOR: led
    let pf = dp.gpiof.split(&mut sc);
    let mut led = pf.pf0.into_output();
    led.set_high().unwrap();
    let on = led.is_set_high().unwrap();
    led.toggle().unwrap();
    let off = led.is_set_low().unwrap();
    // ANCHOR_END: led
    writeln!(uart, "PF0: после set_high {}, после toggle низкий {}", on, off).ok();

    // ANCHOR: hc595
    let pb = dp.gpiob.split(&mut sc);
    let mut leds = Hc595::new(pb.pb0.into_output(), pb.pb1.into_output(), pb.pb2.into_output());
    // Ошибка выводов HAL — Infallible: образец `Ok(())` неопровержим, и
    // обрабатывать нечего.
    let Ok(()) = leds.write(0xA5);
    // ANCHOR_END: hc595
    // SAFETY: только чтение DATA через PTR: выводы порта B заняты
    // драйвером, но чтение их не меняет.
    let port_b = unsafe { (*pac::Gpiob::ptr()).data(0xFF).read().bits() };
    writeln!(uart, "74HC595 <- 0xa5; выводы PB2..PB0 после write: {:03b}", port_b & 0b111).ok();

    // ANCHOR: delay
    let mut delay = Delay::new(cp.SYST);
    let before = pac::SYST::get_current();
    delay.delay_us(100);
    let after = pac::SYST::get_current();
    // ANCHOR_END: delay
    // Сколько именно прошло, в QEMU зависит от загрузки хоста: SysTick там
    // идёт по часам хоста. Гарантия DelayNs — «не меньше».
    let passed = before.wrapping_sub(after) & 0xFF_FFFF;
    writeln!(uart, "delay_us(100): прошло не меньше 1200 тактов: {}", passed >= 1200).ok();
    uart.flush().ok();
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
