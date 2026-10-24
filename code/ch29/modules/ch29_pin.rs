// SPDX-License-Identifier: GPL-2.0

//! 29.5: закреплённая инициализация. Мьютекс ядра нельзя перемещать после
//! инициализации — его поля ссылаются на него самого, — поэтому его не
//! создают значением, а инициализируют на месте: в куче или на стеке.

use kernel::{new_mutex, prelude::*, sync::Mutex};
use pin_init::stack_try_pin_init;

module! {
    type: PinDemo,
    name: "ch29_pin",
    authors: ["Rust textbook"],
    description: "Pinned initialization",
    license: "GPL",
}

// ANCHOR: journal
#[pin_data]
struct Journal {
    name: &'static str,
    #[pin]
    entries: Mutex<KVec<u32>>,
}

impl Journal {
    /// Не значение, а способ инициализировать значение там, куда укажут.
    fn new(name: &'static str) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            name,
            entries <- new_mutex!(KVec::new(), "ch29_pin::entries"),
        })
    }

    fn add(&self, v: u32) -> Result {
        self.entries.lock().push(v, GFP_KERNEL)?;
        Ok(())
    }
}
// ANCHOR_END: journal

/// Адрес числом: `{:p}` ядра 7.0 печатает адрес временной ссылки (29.4).
fn addr<T>(r: &T) -> usize {
    core::ptr::from_ref(r) as usize
}

struct PinDemo {
    journal: Pin<KBox<Journal>>,
}

// ANCHOR: init
impl kernel::Module for PinDemo {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let journal = KBox::pin_init(Journal::new("кадры"), GFP_KERNEL)?;
        pr_info!("{}: мьютекс в куче по адресу {:#x}\n", journal.name, addr(&journal.entries));
        journal.add(1)?;
        journal.add(2)?;

        // Перемещается указатель, а не журнал: адрес мьютекса тот же.
        let moved = journal;
        pr_info!("после перемещения Pin<KBox>: {:#x}\n", addr(&moved.entries));

        stack_try_pin_init!(let local: Journal =? Journal::new("на стеке"));
        local.add(7)?;
        pr_info!(
            "{}: мьютекс по адресу {:#x}, записи {:?}\n",
            local.name,
            addr(&local.entries),
            &*local.entries.lock()
        );
        Ok(PinDemo { journal: moved })
    }
}
// ANCHOR_END: init

impl Drop for PinDemo {
    fn drop(&mut self) {
        pr_info!("{}: записи {:?}\n", self.journal.name, &*self.journal.entries.lock());
    }
}
