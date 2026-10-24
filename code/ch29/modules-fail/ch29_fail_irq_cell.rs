// SPDX-License-Identifier: GPL-2.0

//! 29.8: обработчик прерывания со счётчиком в `Cell`. Обработчик
//! выполняется на любом процессоре одновременно с остальным драйвером,
//! трейт `irq::Handler` требует `Sync`, а `Cell` не `Sync` (E0277).

use core::cell::Cell;
use kernel::{
    device::{Bound, Device},
    irq::{self, IrqReturn},
    prelude::*,
};

module! {
    type: Fail,
    name: "ch29_fail_irq_cell",
    authors: ["Rust textbook"],
    description: "Must not compile",
    license: "GPL",
}

// ANCHOR: cell
struct CountingIrq {
    count: Cell<u32>,
}

impl irq::Handler for CountingIrq {
    fn handle(&self, _dev: &Device<Bound>) -> IrqReturn {
        self.count.set(self.count.get() + 1);
        IrqReturn::Handled
    }
}
// ANCHOR_END: cell

struct Fail;

impl kernel::Module for Fail {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let h = CountingIrq { count: Cell::new(0) };
        pr_info!("{}\n", h.count.get());
        Ok(Fail)
    }
}
