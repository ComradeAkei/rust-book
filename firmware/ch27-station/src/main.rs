//! Глава 27, сквозной проект (раздел 27.9): станция на HAL. TIMER0 каждые
//! 10 мс запускает АЦП, обработчик ADC0SS3 кладёт отсчёт в SPSC-очередь из
//! главы 21, main забирает отсчёты, кодирует кадры telemetry-frame и
//! отправляет их в UART0. Отчёт — через semihosting, чтобы не смешивать
//! его с двоичными кадрами.
//!
//! ```text
//! cd firmware && cargo build --release -p ch27-station
//! qemu-system-arm -cpu cortex-m3 -machine lm3s6965evb -nographic \
//!     -semihosting-config enable=on,target=native -serial file:frames.bin \
//!     -kernel target/thumbv7m-none-eabi/release/ch27-station
//! ```
#![no_std]
#![no_main]

use ch21::spsc::Spsc;
use ch27_hal::adc::Adc;
use ch27_hal::pac::{self, Interrupt, interrupt};
use ch27_hal::prelude::*;
use ch27_hal::serial::{self, Serial};
use ch27_hal::sysctl::{CLOCK_HZ, Sysctl};
use ch27_hal::timer::Timer;
use core::cell::RefCell;
use core::num::NonZeroU16;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use critical_section::Mutex;
use telemetry_frame::{HEADER_LEN, Kind, Measurement, encode};

type Timer0 = Timer<0x4003_0000>;

/// Период выборки.
const PERIOD_MS: u32 = 10;
/// Сколько кадров отправить до выхода из QEMU.
const FRAMES: u32 = 20;

// ANCHOR: queue
/// Отсчёт с меткой времени: его кладёт обработчик и забирает main.
#[derive(Clone, Copy)]
struct Sample {
    t_ms: u32,
    raw: u16,
}

/// Писатель — обработчик ADC0SS3, читатель — main. Очередь из главы 21
/// обходится атомарными load/store: ни критических секций, ни запрета
/// прерываний.
static QUEUE: Spsc<Sample, 8> = Spsc::new();
/// АЦП нужен только обработчику, но настраивает его main.
static ADC: Mutex<RefCell<Option<Adc>>> = Mutex::new(RefCell::new(None));
static SAMPLES: AtomicU32 = AtomicU32::new(0);
static DROPPED: AtomicU32 = AtomicU32::new(0);

#[interrupt]
fn ADC0SS3() {
    let raw = critical_section::with(|cs| ADC.borrow_ref_mut(cs).as_mut().and_then(Adc::take));
    let Some(raw) = raw else { return };
    let n = SAMPLES.fetch_add(1, Relaxed);
    if QUEUE.push(Sample { t_ms: n * PERIOD_MS, raw }).is_err() {
        DROPPED.fetch_add(1, Relaxed);
    }
}
// ANCHOR_END: queue

// ANCHOR: millivolts
/// 10-битный отсчёт в милливольты: опорное напряжение АЦП lm3s6965 — 3 В.
fn millivolts(raw: u16) -> i32 {
    (i32::from(raw) * 3000) >> 10
}
// ANCHOR_END: millivolts

#[entry]
fn main() -> ! {
    // ANCHOR: setup
    let dp = pac::Peripherals::take().unwrap();
    let mut sc = Sysctl::new(dp.sysctl);
    let pa = dp.gpioa.split(&mut sc);
    let pins = (pa.pa0.into_alternate(), pa.pa1.into_alternate());
    let mut uart = Serial::new(dp.uart0, pins, serial::baud(115_200), &mut sc);

    let adc = Adc::on_timer(dp.adc0, &mut sc);
    critical_section::with(|cs| ADC.borrow(cs).replace(Some(adc)));
    let mut timer = Timer0::periodic(dp.timer0, CLOCK_HZ / 1000 * PERIOD_MS, &mut sc);
    timer.trigger_adc();
    // SAFETY: обработчик ADC0SS3 готов, АЦП уже лежит в ADC.
    unsafe { NVIC::unmask(Interrupt::ADC0SS3) };
    timer.start();
    // ANCHOR_END: setup

    // ANCHOR: loop
    let mut seq = 0u8;
    let (mut sent, mut bytes, mut lo, mut hi) = (0u32, 0usize, i32::MAX, i32::MIN);
    let sensor = NonZeroU16::new(1).unwrap();
    while sent < FRAMES {
        let Some(s) = QUEUE.pop() else {
            // Очередь пуста: спать до следующего прерывания.
            cortex_m::asm::wfi();
            continue;
        };
        let m = Measurement { sensor, timestamp_ms: s.t_ms, value: millivolts(s.raw) };
        let mut buf = [0u8; HEADER_LEN + Measurement::WIRE_LEN];
        let Ok(n) = encode(Kind::Measurement, seq, &m.to_bytes(), &mut buf) else {
            hprintln!("кадр не поместился");
            debug::exit(debug::EXIT_FAILURE);
            loop {}
        };
        // Ошибка UART у HAL — Infallible.
        let Ok(()) = uart.write_all(&buf[..n]);
        (lo, hi) = (lo.min(m.value), hi.max(m.value));
        bytes += n;
        sent += 1;
        seq = seq.wrapping_add(1);
    }
    let Ok(()) = uart.flush();
    timer.stop();
    // ANCHOR_END: loop
    hprintln!("станция: {} кадров измерений, {} байт в UART0, {}..{} мВ", sent, bytes, lo, hi);
    hprintln!(
        "отсчётов АЦП {}, потеряно в очереди {}",
        SAMPLES.load(Relaxed),
        DROPPED.load(Relaxed)
    );
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
