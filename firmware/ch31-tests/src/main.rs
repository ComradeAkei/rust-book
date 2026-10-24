//! Глава 31, раздел 31.2: тесты без std на Cortex-M3 в QEMU.
//!
//! libtest на голом железе нет, поэтому прошивка — свой маленький
//! исполнитель: список функций, отчёт через semihosting в формате
//! libtest и код выхода QEMU как вердикт. Два набора тестов: векторы
//! кадров проекта (те же байты, что на хосте) и логика прерывания АЦП из
//! `ch31-irq`, где прерывание — настоящее исключение SysTick, поставленное
//! в ожидание перед выбранным обращением основного цикла.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch31-tests
//! ```
#![no_std]
#![no_main]

use ch31_irq::{Cell, Outcome, Shared, on_sample, take, take_v1};
use core::num::NonZeroU16;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering::Relaxed};
use cortex_m::asm;
use cortex_m::peripheral::SCB;
use cortex_m::peripheral::scb::VectActive;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};
use telemetry_frame::{Kind, Measurement, decode, encode};

// ANCHOR: hw
/// Ячейки в памяти и счётчик обращений основного цикла. Перед обращением
/// номер `fire_at` прошивка ставит SysTick в ожидание: если прерывания
/// разрешены, обработчик выполняется сразу, если запрещены — когда их
/// разрешат.
struct Hw {
    sum: AtomicU32,
    count: AtomicU32,
    accesses: AtomicUsize,
    fire_at: AtomicUsize,
}

static HW: Hw = Hw {
    sum: AtomicU32::new(0),
    count: AtomicU32::new(0),
    accesses: AtomicUsize::new(0),
    fire_at: AtomicUsize::new(usize::MAX),
};

impl Hw {
    fn tick(&self) {
        // Обращения самого обработчика не считаются.
        if SCB::vect_active() != VectActive::ThreadMode {
            return;
        }
        let n = self.accesses.load(Relaxed);
        self.accesses.store(n + 1, Relaxed);
        if n == self.fire_at.load(Relaxed) {
            SCB::set_pendst();
            // Запись в ICSR дошла, конвейер перезагружен: исключение,
            // если разрешено, берётся до следующей инструкции.
            asm::dsb();
            asm::isb();
        }
    }

    fn cell(&self, c: Cell) -> &AtomicU32 {
        match c {
            Cell::Sum => &self.sum,
            Cell::Count => &self.count,
        }
    }
}

impl Shared for Hw {
    fn load(&self, c: Cell) -> u32 {
        self.tick();
        self.cell(c).load(Relaxed)
    }
    fn store(&self, c: Cell, v: u32) {
        self.tick();
        self.cell(c).store(v, Relaxed);
    }
    fn critical<R>(&self, f: impl FnOnce() -> R) -> R {
        cortex_m::interrupt::free(|_| f())
    }
}

/// «Прерывание АЦП»: отсчёт 7.
#[exception]
fn SysTick() {
    on_sample(&HW, 7);
}
// ANCHOR_END: hw

// ANCHOR: scenario
/// Тот же сценарий, что в tests/preempt.rs на хосте: накоплено 100 за
/// 10 отсчётов, прерывание придёт перед обращением `fire_at`.
fn run(take: fn(&Hw) -> (u32, u32), fire_at: usize) -> Outcome {
    HW.sum.store(100, Relaxed);
    HW.count.store(10, Relaxed);
    HW.accesses.store(0, Relaxed);
    HW.fire_at.store(fire_at, Relaxed);
    let taken = take(&HW);
    if HW.accesses.load(Relaxed) <= fire_at {
        // Прерывание не пришло во время take — пусть придёт после.
        SCB::set_pendst();
        asm::dsb();
        asm::isb();
    }
    HW.fire_at.store(usize::MAX, Relaxed);
    Outcome { taken, left: (HW.sum.load(Relaxed), HW.count.load(Relaxed)) }
}

fn take_v1_loses_samples() -> bool {
    let mut ok = true;
    for k in 0..=4 {
        let o = run(take_v1, k);
        hprintln!("    take_v1, прерывание перед обращением {}: {:?}", k, o);
        ok &= o.conserves(107, 11) == !(1..=3).contains(&k);
    }
    ok
}

fn take_in_critical_section_conserves() -> bool {
    (0..=4).all(|k| run(take, k).conserves(107, 11))
}
// ANCHOR_END: scenario

/// Байты кадра на Cortex-M3 те же, что на хосте (вектор из
/// tests-shared/frame.rs).
fn frame_vectors() -> bool {
    const MEAS: [u8; 18] = [
        0xa5, 0x01, 0x01, 0x05, 0x0a, 0x00, 0xe5, 0x73, 0x07, 0x00, 0xe8, 0x03, 0x00, 0x00, 0xd6,
        0xff, 0xff, 0xff,
    ];
    let m = Measurement { sensor: NonZeroU16::new(7).unwrap(), timestamp_ms: 1000, value: -42 };
    let mut buf = [0u8; 18];
    let n = encode(Kind::Measurement, 5, &m.to_bytes(), &mut buf);
    n == Ok(18)
        && buf == MEAS
        && decode(&MEAS).is_ok_and(|f| Measurement::from_bytes(f.payload) == Ok(m))
}

fn crc_check_value() -> bool {
    telemetry_frame::crc16(core::hint::black_box(b"123456789")) == 0x29B1
}

// ANCHOR: runner
/// Тест — имя и функция, которая отвечает, прошёл ли он.
type Test = (&'static str, fn() -> bool);

const TESTS: &[Test] = &[
    ("crc_check_value", crc_check_value),
    ("frame_vectors", frame_vectors),
    ("take_v1_loses_samples", take_v1_loses_samples),
    ("take_in_critical_section_conserves", take_in_critical_section_conserves),
];

#[entry]
fn main() -> ! {
    hprintln!("running {} tests", TESTS.len());
    let mut failed = 0;
    for (name, test) in TESTS {
        let ok = test();
        hprintln!("test {} ... {}", name, if ok { "ok" } else { "FAILED" });
        failed += usize::from(!ok);
    }
    let verdict = if failed == 0 { "ok" } else { "FAILED" };
    hprintln!("test result: {}. {} passed; {} failed", verdict, TESTS.len() - failed, failed);
    debug::exit(if failed == 0 { debug::EXIT_SUCCESS } else { debug::EXIT_FAILURE });
    loop {}
}
// ANCHOR_END: runner

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    hprintln!("паника: {}", info);
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
