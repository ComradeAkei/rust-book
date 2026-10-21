//! Глава 6 на Cortex-M3: те же типы, что в `examples/sizes.rs`, но размеры
//! посчитаны для thumbv7m-none-eabi и выведены через UART PL011.
//!
//! ```text
//! cd firmware && cargo run --release -p ch06-uart
//! ```
#![no_std]
#![no_main]

use ch06::enums::{Message, MessageC, MessageU8};
use ch06::layout::{Record, RecordC, Sample, SampleC, SampleSorted};
use ch06::niche::{Reading, SensorId};
use ch06::packed::WireSample;
use ch06_regs::Pl011;
use core::fmt::Write;
use core::mem::{align_of, size_of};
use core::num::NonZeroU32;
use cortex_m_rt::entry;
use cortex_m_semihosting::debug;

// ANCHOR: writer
/// Адаптер `core::fmt::Write` поверх драйвера: после него работают
/// `write!` и `writeln!` без единой аллокации.
struct Uart(&'static Pl011);

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s);
        Ok(())
    }
}
// ANCHOR_END: writer

macro_rules! row {
    ($out:expr, $($t:ty),* $(,)?) => {
        $( writeln!($out, "{:<28} {:>5} {:>6}", stringify!($t), size_of::<$t>(), align_of::<$t>()).ok(); )*
    };
}

#[entry]
fn main() -> ! {
    // SAFETY: на lm3s6965evb по этому адресу UART0, и больше его никто не трогает.
    let mut out = Uart(unsafe { Pl011::at(Pl011::UART0_BASE) });
    writeln!(out, "ch06 на Cortex-M3, цель thumbv7m-none-eabi").ok();
    writeln!(out, "{:<28} {:>5} {:>6}", "тип", "size", "align").ok();
    row!(
        out,
        Sample,
        SampleC,
        SampleSorted,
        WireSample,
        Record,
        RecordC,
        Message,
        MessageC,
        MessageU8,
        Option<SensorId>,
        Option<Reading>,
        Option<NonZeroU32>,
        Option<u32>,
        Option<&u8>,
        Option<char>,
        usize,
        &[u8],
    );
    writeln!(out, "offsets Sample: {:?}", ch06::layout::sample_offsets()).ok();
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
