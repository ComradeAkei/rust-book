//! Раздел 17.5: `volatile` и MMIO. За адресом регистра устройства — не
//! память, а железо: каждое чтение и запись имеет побочный эффект, и
//! компилятор не вправе их выбрасывать или сливать. Это и обещает
//! `read_volatile`/`write_volatile`. Чего они НЕ дают: ни атомарности, ни
//! упорядочивания обычных доступов вокруг — для последнего есть
//! `compiler_fence`.
//!
//! volatile-операции — `unsafe`; вне блока это ошибка компиляции:
//!
//! ```compile_fail,E0133
//! let x = 0u32;
//! let _ = core::ptr::read_volatile(&x); // требует unsafe
//! ```

use core::ptr;
use core::sync::atomic::{Ordering, compiler_fence};

// ANCHOR: reg
/// Обёртка над регистром MMIO: фиксированный адрес, обращения только
/// volatile. Безопасный снаружи интерфейс (`read`/`write`), весь `unsafe`
/// заперт внутри и опирается на предусловие конструктора.
pub struct Reg {
    addr: *mut u32,
}

impl Reg {
    /// # Safety
    ///
    /// `addr` — валидный, выровненный на 4 адрес регистра MMIO, который
    /// живёт всё время существования `Reg`, и к которому не обращаются
    /// иначе как через эту обёртку.
    pub const unsafe fn new(addr: *mut u32) -> Self {
        Reg { addr }
    }

    pub fn read(&self) -> u32 {
        // SAFETY: addr валиден и выровнен по предусловию `new`;
        // volatile-чтение не выбросить и не слить с соседним.
        unsafe { ptr::read_volatile(self.addr) }
    }

    pub fn write(&self, value: u32) {
        // SAFETY: см. `read`; volatile-запись не сливается с соседней.
        unsafe { ptr::write_volatile(self.addr, value) }
    }
}
// ANCHOR_END: reg

// ANCHOR: fence
/// Заполнить буфер и только потом поднять флаг «готово» для DMA. volatile
/// упорядочивает обращения к самому флагу, но не запрещает компилятору
/// переставить обычные записи буфера за запись флага — это запрещает
/// `compiler_fence(Release)`.
pub fn publish(buf: &mut [u32], flag: &Reg) {
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = i as u32;
    }
    // Все записи буфера должны лечь до записи флага.
    compiler_fence(Ordering::Release);
    flag.write(1);
}
// ANCHOR_END: fence

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reg_roundtrip() {
        let mut cell = 0u32;
        // SAFETY: cell жива всю область видимости и доступна только через reg.
        let reg = unsafe { Reg::new(&mut cell) };
        reg.write(0xBEEF);
        assert_eq!(reg.read(), 0xBEEF);
    }

    #[test]
    fn publish_fills_then_flags() {
        let mut cell = 0u32;
        // SAFETY: cell доступна только через flag на время теста.
        let flag = unsafe { Reg::new(&mut cell) };
        let mut buf = [0u32; 4];
        publish(&mut buf, &flag);
        assert_eq!(buf, [0, 1, 2, 3]);
        assert_eq!(flag.read(), 1);
    }
}
