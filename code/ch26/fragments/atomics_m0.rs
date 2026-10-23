// Раздел 26.1: alloc и core есть на Cortex-M0, но без атомарного
// чтения-изменения-записи: у ARMv6-M нет ldrex/strex.
// ANCHOR: atomics_m0
#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};

pub fn shared(x: u32) -> Arc<u32> {
    Arc::new(x)
}

pub fn bump(c: &AtomicU32) -> u32 {
    c.fetch_add(1, Ordering::Relaxed)
}
// ANCHOR_END: atomics_m0
