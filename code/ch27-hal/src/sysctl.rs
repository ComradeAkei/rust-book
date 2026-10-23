//! Такты периферии. Пока бит блока в регистре RCGCn равен нулю, блок не
//! тактируется: на кристалле обращение к его регистрам — отказ шины.
//! QEMU такты не проверяет, но HAL включает их, как требует техническое
//! описание.

use crate::pac;

// ANCHOR: clock
/// Частота ядра после сброса: внутренний генератор 12 МГц (поле OSCSRC
/// регистра RCC). PLL HAL не включает; SysTick в QEMU идёт с той же
/// частотой (раздел 27.1).
pub const CLOCK_HZ: u32 = 12_000_000;
// ANCHOR_END: clock

/// Блок, которому нужны такты.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gate {
    /// Порт GPIO: 0 — A, …, 6 — G.
    Gpio(u8),
    Uart(u8),
    Timer(u8),
    Adc,
}

/// Владелец регистров системного управления: включать такты может
/// только тот, у кого `&mut Sysctl`.
pub struct Sysctl {
    regs: pac::Sysctl,
}

impl Sysctl {
    pub fn new(regs: pac::Sysctl) -> Sysctl {
        Sysctl { regs }
    }

    // ANCHOR: enable
    pub fn enable(&mut self, gate: Gate) {
        let r = &self.regs;
        match gate {
            Gate::Gpio(n) => r.rcgc2().modify(|_, w| w.gpio(n).set_bit()),
            Gate::Uart(n) => r.rcgc1().modify(|_, w| w.uart(n).set_bit()),
            Gate::Timer(n) => r.rcgc1().modify(|_, w| w.timer(n).set_bit()),
            Gate::Adc => r.rcgc0().modify(|_, w| w.adc().set_bit()),
        };
        // Блок оживает через три такта после записи: чтение по шине
        // периферии их и занимает.
        let _ = r.rcgc2().read();
    }
    // ANCHOR_END: enable

    pub fn free(self) -> pac::Sysctl {
        self.regs
    }
}
