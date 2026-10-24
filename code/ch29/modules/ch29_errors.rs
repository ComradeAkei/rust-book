// SPDX-License-Identifier: GPL-2.0

//! 29.3: три ресурса в `init` и откат на любом шаге. Параметр `fail_at`
//! имитирует отказ на шаге 1–4; ресурсы, полученные до отказа,
//! освобождает сам `try_pin_init!` — в обратном порядке.

use kernel::{
    debugfs,
    fs::File,
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    prelude::*,
};

module! {
    type: Errors,
    name: "ch29_errors",
    authors: ["Rust textbook"],
    description: "Error paths without goto",
    license: "GPL",
    params: {
        fail_at: u32 {
            default: 0,
            description: "Fail at step 1-4 (0: do not fail)",
        },
    },
}

// ANCHOR: logged
/// Ресурс, который сообщает в журнал, когда его освобождают.
struct Logged<T> {
    name: &'static str,
    _res: T,
}

impl<T> Logged<T> {
    fn new(name: &'static str, res: T) -> Self {
        Logged { name, _res: res }
    }
}

impl<T> Drop for Logged<T> {
    fn drop(&mut self) {
        pr_info!("освобождаю: {}\n", self.name);
    }
}
// ANCHOR_END: logged

/// Отказ на шаге `step`, если так велит параметр.
fn inject(step: u32, err: Error) -> Result {
    if *module_parameters::fail_at.value() == step {
        pr_info!("шаг {step}: отказ {err:?}\n");
        return Err(err);
    }
    Ok(())
}

/// Устройство без операций: важна только регистрация.
struct Dummy;

#[vtable]
impl MiscDevice for Dummy {
    type Ptr = ();

    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result {
        Ok(())
    }
}

/// Регистрация misc-устройства, которая сообщает о своём снятии.
#[pin_data(PinnedDrop)]
struct LoggedMisc {
    #[pin]
    reg: MiscDeviceRegistration<Dummy>,
}

#[pinned_drop]
impl PinnedDrop for LoggedMisc {
    fn drop(self: Pin<&mut Self>) {
        pr_info!("освобождаю: /dev/ch29-errors\n");
    }
}

// ANCHOR: init
#[pin_data]
struct Errors {
    buf: Logged<KVec<u8>>,
    #[pin]
    misc: LoggedMisc,
    dir: Logged<debugfs::Dir>,
}

impl kernel::InPlaceModule for Errors {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            buf: {
                inject(1, ENOMEM)?;
                Logged::new("буфер", KVec::with_capacity(4096, GFP_KERNEL)?)
            },
            misc <- {
                inject(2, EBUSY)?;
                let opts = MiscDeviceOptions { name: c"ch29-errors" };
                try_pin_init!(LoggedMisc { reg <- MiscDeviceRegistration::register(opts) })
            },
            dir: {
                inject(3, ENOENT)?;
                Logged::new("каталог debugfs", debugfs::Dir::new(c"ch29_errors"))
            },
            _: {
                inject(4, EINVAL)?;
                pr_info!("загружен: буфер, /dev/ch29-errors, debugfs\n");
            },
        })
    }
}
// ANCHOR_END: init
