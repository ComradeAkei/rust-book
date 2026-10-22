//! Глава 3, раздел 3.6: страж критической секции на Cortex-M3.
//!
//! Прерывание SysTick ставится в ожидание программно (`SCB::set_pendst`).
//! Если прерывания разрешены, обработчик выполняется сразу и увеличивает
//! счётчик. Если запрещены, он ждёт, пока страж не будет уничтожен.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch03-irq
//! ```
#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m::interrupt;
use cortex_m::peripheral::SCB;
use cortex_m::register::primask;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};

static TICKS: AtomicU32 = AtomicU32::new(0);

#[exception]
fn SysTick() {
    // fetch_add на Cortex-M0 нет (там нет LDREX/STREX), а пишет в счётчик
    // только этот обработчик, поэтому хватает чтения и записи.
    TICKS.store(TICKS.load(Relaxed) + 1, Relaxed);
}

// ANCHOR: guard
/// Критическая секция: пока страж жив, прерывания запрещены. Деструктор
/// возвращает PRIMASK в то состояние, в котором его застал конструктор,
/// поэтому вложенные секции работают правильно.
pub struct IrqGuard {
    was_enabled: bool,
}

// `Default` для стража не нужен: `IrqGuard::default()` прятал бы за
// безобидным именем запрет прерываний.
#[allow(clippy::new_without_default)]
impl IrqGuard {
    #[must_use = "страж без имени уничтожается сразу, и секция кончается, не начавшись"]
    pub fn new() -> Self {
        // В cortex-m `Primask::Active` значит «прерывания разрешены»: бит
        // PRIMASK сброшен.
        let was_enabled = primask::read().is_active();
        interrupt::disable();
        IrqGuard { was_enabled }
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        if self.was_enabled {
            // SAFETY: разрешаем прерывания, только если они были разрешены
            // до конструктора: секция не может открыть чужую секцию.
            unsafe { interrupt::enable() };
        }
    }
}
// ANCHOR_END: guard

// ANCHOR: naive
/// Наивный страж: в деструкторе всегда разрешает прерывания.
pub struct NaiveGuard;

#[allow(clippy::new_without_default)]
impl NaiveGuard {
    #[must_use]
    pub fn new() -> Self {
        interrupt::disable();
        NaiveGuard
    }
}

impl Drop for NaiveGuard {
    fn drop(&mut self) {
        // SAFETY: никакого: так делать нельзя, см. опыт с вложенной секцией.
        unsafe { interrupt::enable() };
    }
}
// ANCHOR_END: naive

/// 1, если прерывания запрещены (бит PRIMASK установлен).
fn primask_bit() -> u32 {
    primask::read().is_inactive() as u32
}

fn ticks() -> u32 {
    TICKS.load(Relaxed)
}

// ANCHOR: main
#[entry]
fn main() -> ! {
    {
        let _g = IrqGuard::new();
        SCB::set_pendst();
        hprintln!("let _g = ...: PRIMASK = {}, тиков {}", primask_bit(), ticks());
    }
    hprintln!("после блока:  PRIMASK = {}, тиков {}", primask_bit(), ticks());

    {
        let _ = IrqGuard::new(); // страж уничтожен на этой же строке
        SCB::set_pendst();
        hprintln!("let _ = ...:  PRIMASK = {}, тиков {}", primask_bit(), ticks());
    }

    {
        let _outer = IrqGuard::new();
        {
            let _inner = IrqGuard::new();
        }
        SCB::set_pendst();
        hprintln!("вложенная, IrqGuard:   PRIMASK = {}, тиков {}", primask_bit(), ticks());
    }
    hprintln!("после внешней:         PRIMASK = {}, тиков {}", primask_bit(), ticks());

    {
        let _outer = NaiveGuard::new();
        {
            let _inner = NaiveGuard::new();
        }
        SCB::set_pendst();
        hprintln!("вложенная, NaiveGuard: PRIMASK = {}, тиков {}", primask_bit(), ticks());
    }

    {
        let outer = IrqGuard::new();
        let _inner = IrqGuard::new();
        drop(outer); // внешний страж уничтожен раньше внутреннего
        SCB::set_pendst();
        hprintln!("drop(outer), inner жив:  PRIMASK = {}, тиков {}", primask_bit(), ticks());
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
