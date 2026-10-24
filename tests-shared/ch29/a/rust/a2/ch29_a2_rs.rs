// SPDX-License-Identifier: GPL-2.0

//! Задача 29.A, фрагмент 2 на Rust — самый близкий к C перевод.

use kernel::prelude::*;

module! {
    type: A2,
    name: "ch29_a2_rs",
    authors: ["Rust textbook"],
    description: "Task 29.A, fragment 2 in Rust",
    license: "GPL",
}

struct Sensor {
    samples: KVec<u32>,
    n: usize,
}

// ANCHOR: frag
fn sensor_remove(s: &mut Sensor) {
    let samples = core::mem::take(&mut s.samples);
    drop(samples);
    pr_info!("последний отсчёт: {}\n", samples[s.n - 1]);
}
// ANCHOR_END: frag

struct A2;

impl kernel::Module for A2 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let mut s = Sensor { samples: KVec::from_elem(0u32, 8, GFP_KERNEL)?, n: 8 };
        s.samples[7] = 42;
        sensor_remove(&mut s);
        Ok(A2)
    }
}
