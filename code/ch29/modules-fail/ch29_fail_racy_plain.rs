// SPDX-License-Identifier: GPL-2.0

//! 29.6: счётчик `counter++` из ch29_sync_c.c без блокировки: через `Arc`
//! работы видят счётчик только по общей ссылке, и писать в него нельзя
//! (E0594).

use kernel::{prelude::*, sync::Arc, workqueue};

module! {
    type: Fail,
    name: "ch29_fail_racy_plain",
    authors: ["Rust textbook"],
    description: "Must not compile",
    license: "GPL",
}

struct Shared {
    plain: u64,
}

struct Fail;

// ANCHOR: plain
impl kernel::Module for Fail {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let shared = Arc::new(Shared { plain: 0 }, GFP_KERNEL)?;
        for _ in 0..4 {
            let s = shared.clone();
            workqueue::system_unbound().try_spawn(GFP_KERNEL, move || {
                for _ in 0..100_000 {
                    s.plain += 1;
                }
            })?;
        }
        Ok(Fail)
    }
}
// ANCHOR_END: plain
