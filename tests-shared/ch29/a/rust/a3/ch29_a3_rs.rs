// SPDX-License-Identifier: GPL-2.0

//! Задача 29.A, фрагмент 3 на Rust — самый близкий к C перевод.

use kernel::{new_mutex, prelude::*, sync::Mutex};

module! {
    type: A3,
    name: "ch29_a3_rs",
    authors: ["Rust textbook"],
    description: "Task 29.A, fragment 3 in Rust",
    license: "GPL",
}

struct Samples {
    samples: [u32; 16],
    count: usize,
}

#[pin_data]
struct Sensor {
    #[pin]
    lock: Mutex<Samples>,
}

// ANCHOR: frag
fn sensor_last(s: &Sensor) -> Result<u32> {
    let guard = s.lock.lock();
    if guard.count == 0 {
        return Err(EAGAIN);
    }
    Ok(guard.samples[guard.count - 1])
}
// ANCHOR_END: frag

struct A3 {
    _sensor: Pin<KBox<Sensor>>,
}

impl kernel::Module for A3 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let sensor = KBox::pin_init(
            pin_init!(Sensor { lock <- new_mutex!(Samples { samples: [0; 16], count: 0 }) }),
            GFP_KERNEL,
        )?;
        pr_info!("sensor_last: {:?}\n", sensor_last(&sensor));
        Ok(A3 { _sensor: sensor })
    }
}
