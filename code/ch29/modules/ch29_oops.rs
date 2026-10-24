// SPDX-License-Identifier: GPL-2.0

//! 29.9: паника в модуле. Запись в /dev/ch29-oops слова `unwrap`,
//! `index` или `overflow` вызывает панику Rust в обработчике write — ядро
//! отвечает oops и убивает пишущий процесс. `locked` паникует с
//! захваченным мьютексом: раскрутки нет, охранник не отпускается, и
//! следующий `locked` находит мьютекс занятым. Это же устройство — цель
//! точки останова в сеансе gdb.

use core::hint::black_box;
use kernel::{
    fs::{File, Kiocb},
    iov::IovIterSource,
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    prelude::*,
};

kernel::sync::global_lock! {
    // SAFETY: инициализируется в `init` модуля до первого использования.
    unsafe(uninit) static LOCKED: Mutex<()> = ();
}

module! {
    type: OopsModule,
    name: "ch29_oops",
    authors: ["Rust textbook"],
    description: "Rust panics become oopses",
    license: "GPL",
}

#[pin_data]
struct OopsModule {
    #[pin]
    _misc: MiscDeviceRegistration<Oops>,
}

impl kernel::InPlaceModule for OopsModule {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        // SAFETY: вызывается один раз, до регистрации устройства.
        unsafe { LOCKED.init() };
        try_pin_init!(Self {
            _misc <- MiscDeviceRegistration::register(MiscDeviceOptions { name: c"ch29-oops" }),
        })
    }
}

struct Oops;

// ANCHOR: trigger
/// Разбирает команду и делает то, что она велит. `#[inline(never)]` —
/// чтобы функция была видна в трассировке стека и в gdb отдельным кадром.
#[inline(never)]
fn trigger(cmd: &[u8]) -> Result<usize> {
    let table = [10u32, 20, 30];
    match cmd {
        b"unwrap" => {
            let found = table.iter().position(|&x| x == black_box(40));
            Ok(found.unwrap())
        }
        b"index" => Ok(table[black_box(5)] as usize),
        b"overflow" => Ok((black_box(u32::MAX) + table[0]) as usize),
        b"locked" => {
            // Охранник жив до конца ветки, а ветка кончается паникой.
            let Some(_guard) = LOCKED.try_lock() else {
                pr_info!("мьютекс занят: охранник прошлой паники не отпущен\n");
                return Err(EBUSY);
            };
            panic!("паника с захваченным мьютексом");
        }
        _ => Err(EINVAL),
    }
}
// ANCHOR_END: trigger

#[vtable]
impl MiscDevice for Oops {
    type Ptr = ();

    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result {
        Ok(())
    }

    fn write_iter(_kiocb: Kiocb<'_, ()>, iov: &mut IovIterSource<'_>) -> Result<usize> {
        let mut cmd = [0u8; 16];
        let n = iov.copy_from_iter(&mut cmd);
        // Без '\n' от echo. `trim_ascii_end` стабилен с rustc 1.80, а ядро
        // 7.0 обещает сборку начиная с 1.78 — CLIPPY=1 следит за этим.
        let end = cmd[..n].iter().rposition(|c| !c.is_ascii_whitespace()).map_or(0, |i| i + 1);
        let cmd = &cmd[..end];
        pr_info!("команда: {}\n", core::str::from_utf8(cmd).unwrap_or("?"));
        trigger(cmd)?;
        Ok(n)
    }
}
