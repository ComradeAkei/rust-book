// SPDX-License-Identifier: GPL-2.0

//! Задача 29.A, фрагмент 4 на Rust — самый близкий к C перевод.

use kernel::{
    new_spinlock,
    prelude::*,
    sync::SpinLock,
    time::{delay::fsleep, Delta},
};

module! {
    type: A4,
    name: "ch29_a4_rs",
    authors: ["Rust textbook"],
    description: "Task 29.A, fragment 4 in Rust",
    license: "GPL",
}

#[pin_data]
struct Sensor {
    #[pin]
    pending: SpinLock<u32>,
}

// ANCHOR: frag
fn sensor_flush(s: &Sensor) {
    let mut pending = s.pending.lock();
    while *pending > 0 {
        fsleep(Delta::from_millis(1)); // устройству нужна миллисекунда на отсчёт
        *pending -= 1;
    }
}
// ANCHOR_END: frag

struct A4;

impl kernel::Module for A4 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let sensor = KBox::pin_init(pin_init!(Sensor { pending <- new_spinlock!(2) }), GFP_KERNEL)?;
        sensor_flush(&sensor);
        Ok(A4)
    }
}
