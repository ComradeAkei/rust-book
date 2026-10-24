// SPDX-License-Identifier: GPL-2.0

//! Задача 29.A, фрагмент 1 на Rust — самый близкий к C перевод.

use kernel::{prelude::*, str::CString};

module! {
    type: A1,
    name: "ch29_a1_rs",
    authors: ["Rust textbook"],
    description: "Task 29.A, fragment 1 in Rust",
    license: "GPL",
}

struct Sensor {
    _name: CString,
    _samples: KVec<u32>,
}

// ANCHOR: frag
fn sensor_setup(name: &CStr, n: usize) -> Result<Sensor> {
    let name = CString::try_from(name)?;
    let samples = KVec::from_elem(0u32, n, GFP_KERNEL)?;
    if n > 64 {
        return Err(EINVAL);
    }
    Ok(Sensor { _name: name, _samples: samples })
}
// ANCHOR_END: frag

struct A1 {
    _sensor: Sensor,
}

impl kernel::Module for A1 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        Ok(A1 { _sensor: sensor_setup(c"термометр", 100)? })
    }
}
