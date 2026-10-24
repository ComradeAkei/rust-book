// SPDX-License-Identifier: GPL-2.0

//! 29.4: плавающая точка в модуле. Цель ядра — без SSE и с soft-float,
//! поэтому `f32` компилируется в вызовы функций вроде `__mulsf3`, а
//! ядро их не экспортирует: модуль не соберёт modpost.

use core::hint::black_box;
use kernel::prelude::*;

module! {
    type: Fail,
    name: "ch29_fail_float",
    authors: ["Rust textbook"],
    description: "Must not link",
    license: "GPL",
}

struct Fail;

// ANCHOR: float
impl kernel::Module for Fail {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let x = black_box(1.5f32) * black_box(2.0f32);
        pr_info!("x = {}\n", x as u32);
        Ok(Fail)
    }
}
// ANCHOR_END: float
