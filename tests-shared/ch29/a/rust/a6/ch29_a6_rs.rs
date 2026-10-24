// SPDX-License-Identifier: GPL-2.0

//! Задача 29.A, фрагмент 6 на Rust — самый близкий к C перевод.

use kernel::{new_mutex, prelude::*, sync::Mutex};

module! {
    type: A6,
    name: "ch29_a6_rs",
    authors: ["Rust textbook"],
    description: "Task 29.A, fragment 6 in Rust",
    license: "GPL",
}

#[pin_data]
struct Sensor {
    #[pin]
    samples: Mutex<u32>,
}

// ANCHOR: frag
fn sensor_move(from: &Sensor, to: &Sensor) {
    let mut f = from.samples.lock();
    let mut t = to.samples.lock();
    *t += *f;
    *f = 0;
}
// ANCHOR_END: frag

struct A6;

impl kernel::Module for A6 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        // Два места new_mutex! — два класса блокировок, как два mutex_init в C.
        let inner = KBox::pin_init(pin_init!(Sensor { samples <- new_mutex!(3) }), GFP_KERNEL)?;
        let outer = KBox::pin_init(pin_init!(Sensor { samples <- new_mutex!(0) }), GFP_KERNEL)?;
        sensor_move(&inner, &outer); // сбор отсчётов
        sensor_move(&outer, &inner); // возврат при сбросе
        Ok(A6)
    }
}
