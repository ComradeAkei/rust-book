//! Раздел 3.4: `mem::replace`, `mem::take`, `mem::swap`, `mem::forget` и
//! `ManuallyDrop`.

use crate::drop_order::{Noisy, log};
use core::mem::{self, ManuallyDrop};
use std::boxed::Box;
use std::vec::Vec;

// ANCHOR: replace
/// Приёмник кадров: состояние владеет буфером, и переход между
/// состояниями передаёт буфер дальше без копирования байтов.
#[derive(Debug, Default, PartialEq)]
pub enum Rx {
    #[default]
    Idle,
    /// Принят байт начала кадра, копим тело.
    Body(Vec<u8>),
    /// Принят байт конца кадра, кадр готов.
    Done(Vec<u8>),
}

pub const START: u8 = 0x7E;
pub const END: u8 = 0x7F;

impl Rx {
    pub fn on_byte(&mut self, b: u8) {
        // Забираем старое состояние по значению, оставляя на его месте Idle.
        *self = match (mem::replace(self, Rx::Idle), b) {
            (Rx::Idle, START) => Rx::Body(Vec::new()),
            (Rx::Idle, _) => Rx::Idle,
            (Rx::Body(buf), END) => Rx::Done(buf),
            (Rx::Body(mut buf), b) => {
                buf.push(b);
                Rx::Body(buf)
            }
            (done @ Rx::Done(_), _) => done,
        };
    }

    /// Забрать готовый кадр. `mem::take` оставляет на месте `Rx::default()`.
    pub fn take_frame(&mut self) -> Option<Vec<u8>> {
        match mem::take(self) {
            Rx::Done(frame) => Some(frame),
            other => {
                *self = other;
                None
            }
        }
    }
}
// ANCHOR_END: replace

// ANCHOR: swap
/// Двойной буфер: в один пишет DMA, другой обрабатывает программа.
pub struct PingPong {
    pub dma: Box<[u8; 512]>,
    pub cpu: Box<[u8; 512]>,
}

/// Меняет буферы местами: два указателя, 512 байт никто не копирует.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_flip(p: &mut PingPong) {
    mem::swap(&mut p.dma, &mut p.cpu);
}

/// Те же буферы без `Box`: `swap` меняет местами по 512 байт.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_flip_inline(a: &mut [u8; 512], b: &mut [u8; 512]) {
    mem::swap(a, b);
}
// ANCHOR_END: swap

// ANCHOR: manually_drop
/// Драйвер должен отпустить шину раньше, чем выключится тактирование, а
/// поля хочется объявить в другом порядке. `ManuallyDrop` отключает
/// автоматический вызов деструктора поля, и порядок задаёт `Drop` структуры.
pub struct Driver {
    pub clock: ManuallyDrop<Noisy>,
    pub bus: ManuallyDrop<Noisy>,
}

impl Drop for Driver {
    fn drop(&mut self) {
        // SAFETY: каждое поле уничтожается ровно один раз, и после этого
        // к нему никто не обращается: структура уничтожается.
        unsafe {
            ManuallyDrop::drop(&mut self.bus);
            ManuallyDrop::drop(&mut self.clock);
        }
    }
}
// ANCHOR_END: manually_drop

// ANCHOR: forget
/// `mem::forget` — безопасная функция: утечка не нарушает безопасность
/// памяти, она просто не возвращает ресурс.
pub fn forget_one() {
    let a = Noisy("a");
    let b = Noisy("b");
    mem::forget(a);
    log("конец");
    drop(b);
}
// ANCHOR_END: forget

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop_order::take_log;

    #[test]
    fn rx_state_machine() {
        let mut rx = Rx::default();
        for b in [0x00, START, 1, 2, 3, END, 9] {
            rx.on_byte(b);
        }
        assert_eq!(rx.take_frame(), Some(vec![1, 2, 3]));
        assert_eq!(rx, Rx::Idle);
        rx.on_byte(START);
        assert_eq!(rx.take_frame(), None);
        assert_eq!(rx, Rx::Body(vec![]));
    }

    #[test]
    fn flip_swaps_pointers() {
        let mut p = PingPong { dma: Box::new([1; 512]), cpu: Box::new([2; 512]) };
        let (d, c) = (p.dma.as_ptr(), p.cpu.as_ptr());
        ch03_flip(&mut p);
        assert_eq!((p.dma.as_ptr(), p.cpu.as_ptr()), (c, d));
        assert_eq!((p.dma[0], p.cpu[0]), (2, 1));
        let (mut a, mut b) = ([1; 512], [2; 512]);
        ch03_flip_inline(&mut a, &mut b);
        assert_eq!((a[511], b[0]), (2, 1));
    }

    #[test]
    fn manual_drop_order() {
        drop(Driver {
            clock: ManuallyDrop::new(Noisy("clock")),
            bus: ManuallyDrop::new(Noisy("bus")),
        });
        assert_eq!(take_log(), ["bus", "clock"]);
    }

    #[test]
    fn forget_skips_destructor() {
        forget_one();
        assert_eq!(take_log(), ["конец", "b"]);
    }

    #[test]
    fn needs_drop() {
        assert!(!mem::needs_drop::<[u8; 512]>());
        assert!(mem::needs_drop::<Box<[u8; 512]>>());
        assert!(!mem::needs_drop::<ManuallyDrop<Noisy>>());
        assert_eq!(size_of::<ManuallyDrop<Noisy>>(), size_of::<Noisy>());
    }
}
