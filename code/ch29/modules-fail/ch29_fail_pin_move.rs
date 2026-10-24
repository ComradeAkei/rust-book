// SPDX-License-Identifier: GPL-2.0

//! 29.5: журнал с мьютексом закреплён в куче — вынуть его оттуда
//! перемещением, как `memcpy` в ch29_pin_c.c, нельзя: `Pin` отдаёт
//! указатель обратно только для типов `Unpin`, а мьютекс ядра не `Unpin`
//! (E0277).

use kernel::{new_mutex, prelude::*, sync::Mutex};

module! {
    type: Fail,
    name: "ch29_fail_pin_move",
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

struct Fail {
    _moved: Journal,
}

// ANCHOR: move
impl kernel::Module for Fail {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let journal = KBox::pin_init(Journal::new(), GFP_KERNEL)?;
        let boxed: KBox<Journal> = Pin::into_inner(journal);
        let moved: Journal = KBox::into_inner(boxed);
        Ok(Fail { _moved: moved })
    }
}
// ANCHOR_END: move
