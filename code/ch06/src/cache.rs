//! Раздел 6.8: `repr(align(N))` и кэш-линии.

use core::ops::{Deref, DerefMut};

// ANCHOR: cache_padded
/// Значение, которое занимает собственную кэш-линию.
///
/// 64 байта верно для x86_64 и большинства ARM Cortex-A. У Cortex-M3 и M4
/// кэша данных нет вовсе, а у Cortex-M7 строка кэша 32 байта.
#[repr(C, align(64))]
#[derive(Default)]
pub struct CachePadded<T>(pub T);

impl<T> Deref for CachePadded<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for CachePadded<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
// ANCHOR_END: cache_padded

// ANCHOR: counters
// 64-битных атомиков нет на Cortex-M: у ARMv6-M и ARMv7-M нет инструкций
// для атомарной работы с 8 байтами. Пример для хоста отключается там cfg.
#[cfg(target_has_atomic = "64")]
use core::sync::atomic::AtomicU64;

/// Два счётчика рядом: оба попадают в одну кэш-линию.
#[cfg(target_has_atomic = "64")]
#[derive(Default)]
pub struct Counters {
    pub produced: AtomicU64,
    pub consumed: AtomicU64,
}

/// Те же счётчики, разнесённые по разным кэш-линиям.
#[cfg(target_has_atomic = "64")]
#[derive(Default)]
pub struct PaddedCounters {
    pub produced: CachePadded<AtomicU64>,
    pub consumed: CachePadded<AtomicU64>,
}
// ANCHOR_END: counters

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    #[test]
    fn padded_layout() {
        assert_eq!(align_of::<CachePadded<u8>>(), 64);
        assert_eq!(size_of::<CachePadded<u8>>(), 64);
        assert_eq!(size_of::<Counters>(), 16);
        assert_eq!(size_of::<PaddedCounters>(), 128);
        // PaddedCounters без repr(C): порядок полей не гарантирован, поэтому
        // сравниваем расстояние, а не разность. Иначе тест сам зависел бы от
        // раскладки repr(Rust) и падал бы под -Zrandomize-layout.
        let produced = offset_of!(PaddedCounters, produced);
        let consumed = offset_of!(PaddedCounters, consumed);
        assert_eq!(produced.abs_diff(consumed), 64);
    }

    #[test]
    fn array_elements_stay_aligned() {
        let arr: [CachePadded<u8>; 3] = Default::default();
        for cell in &arr {
            assert_eq!((cell as *const _ as usize) % 64, 0);
        }
    }
}
