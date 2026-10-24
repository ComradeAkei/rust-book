// SPDX-License-Identifier: GPL-2.0

//! Задача 29.C: счётчик /dev/ch29-counter, общий для всех открытий, под
//! мьютексом ядра. Поведение — как у эталона на C
//! (tests-shared/ch29/c-ref/ch29_counter_c.c): тест прогоняет оба одним
//! сеансом и сравнивает вывод. Заготовка регистрирует устройство, но
//! read и write пока отвечают ENOSYS.

use kernel::{
    fs::{File, Kiocb},
    iov::{IovIterDest, IovIterSource},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    prelude::*,
};

module! {
    type: CounterModule,
    name: "ch29_c",
    authors: ["Rust textbook"],
    description: "Task 29.C: shared counter under a kernel mutex",
    license: "GPL",
}

#[pin_data]
struct CounterModule {
    #[pin]
    _misc: MiscDeviceRegistration<Reader>,
}

impl kernel::InPlaceModule for CounterModule {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            _misc <- MiscDeviceRegistration::register(MiscDeviceOptions { name: c"ch29-counter" }),
        })
    }
}

/// Состояние одного открытия.
struct Reader;

/// «Не реализовано»: константы ENOSYS в крейте kernel 7.0 нет.
fn not_done() -> Error {
    Error::from_errno(-(kernel::bindings::ENOSYS as i32))
}

#[vtable]
impl MiscDevice for Reader {
    type Ptr = Pin<KBox<Self>>;

    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result<Pin<KBox<Self>>> {
        Ok(KBox::pin(Reader, GFP_KERNEL)?)
    }

    fn read_iter(_kiocb: Kiocb<'_, Self::Ptr>, _iov: &mut IovIterDest<'_>) -> Result<usize> {
        Err(not_done())
    }

    fn write_iter(_kiocb: Kiocb<'_, Self::Ptr>, _iov: &mut IovIterSource<'_>) -> Result<usize> {
        Err(not_done())
    }
}
