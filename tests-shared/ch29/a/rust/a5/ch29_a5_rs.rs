// SPDX-License-Identifier: GPL-2.0

//! Задача 29.A, фрагмент 5 на Rust — самый близкий к C перевод.

use kernel::prelude::*;

module! {
    type: A5,
    name: "ch29_a5_rs",
    authors: ["Rust textbook"],
    description: "Task 29.A, fragment 5 in Rust",
    license: "GPL",
}

struct Sensor {
    samples: KVec<u32>,
}

// ANCHOR: frag
fn sensor_store(s: &mut Sensor, i: usize, v: u32) -> Result {
    if i > s.samples.len() {
        return Err(EINVAL);
    }
    s.samples[i] = v;
    Ok(())
}
// ANCHOR_END: frag

struct A5;

impl kernel::Module for A5 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let mut s = Sensor { samples: KVec::from_elem(0u32, 8, GFP_KERNEL)? };
        for i in 0..=s.samples.len() {
            sensor_store(&mut s, i, i as u32 * 10)?;
        }
        Ok(A5)
    }
}
