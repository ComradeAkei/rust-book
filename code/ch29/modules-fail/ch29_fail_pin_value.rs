// SPDX-License-Identifier: GPL-2.0

//! 29.5: `Journal::new` возвращает не журнал, а инициализатор — получить
//! из него значение на стеке нельзя (E0308).

use kernel::{new_mutex, prelude::*, sync::Mutex};

module! {
    type: Fail,
    name: "ch29_fail_pin_value",
    authors: ["Rust textbook"],
    description: "Must not compile",
    license: "GPL",
}

#[pin_data]
struct Journal {
    #[pin]
    entries: Mutex<KVec<u32>>,
}

impl Journal {
    fn new() -> impl PinInit<Self, Error> {
        try_pin_init!(Self { entries <- new_mutex!(KVec::new()) })
    }
}

struct Fail;

// ANCHOR: value
impl kernel::Module for Fail {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let journal: Journal = Journal::new();
        journal.entries.lock().push(1, GFP_KERNEL)?;
        Ok(Fail)
    }
}
// ANCHOR_END: value
