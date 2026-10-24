// SPDX-License-Identifier: GPL-2.0

//! 29.6: обход через `Cell` — запись по общей ссылке без блокировки.
//! В одном потоке это законно, но `Cell` не `Sync`, и работа в очереди
//! ядра, которая выполняется на другом процессоре, её не получит (E0277).

use core::cell::Cell;
use kernel::{prelude::*, sync::Arc, workqueue};

module! {
    type: Fail,
    name: "ch29_fail_racy_cell",
    authors: ["Rust textbook"],
    description: "Must not compile",
    license: "GPL",
}

struct Fail;

// ANCHOR: cell
impl kernel::Module for Fail {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let counter = Arc::new(Cell::new(0u64), GFP_KERNEL)?;
        for _ in 0..4 {
            let c = counter.clone();
            workqueue::system_unbound().try_spawn(GFP_KERNEL, move || {
                for _ in 0..100_000 {
                    c.set(c.get() + 1);
                }
            })?;
        }
        Ok(Fail)
    }
}
// ANCHOR_END: cell
