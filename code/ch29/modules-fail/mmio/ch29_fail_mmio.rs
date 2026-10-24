// SPDX-License-Identifier: GPL-2.0

//! 29.8: регистр за концом BAR. Смещение известно при компиляции, и
//! проверка `build_assert!` в `Io::read32` оставляет в коде вызов
//! `rust_build_error` — функции, которой нет: модуль не соберёт modpost.

use kernel::{device::Core, io::Io, pci, prelude::*};

// SAFETY: как в ch29_edu.rs: `pci::Vendor` — `#[repr(transparent)]` над u16.
const QEMU_VENDOR: pci::Vendor = unsafe { core::mem::transmute::<u16, pci::Vendor>(0x1234) };

kernel::pci_device_table!(
    PCI_TABLE,
    MODULE_PCI_TABLE,
    <Fail as pci::Driver>::IdInfo,
    [(pci::DeviceId::from_id(QEMU_VENDOR, 0x11e8), ())]
);

#[pin_data]
struct Fail {
    past: u32,
}

impl pci::Driver for Fail {
    type IdInfo = ();
    const ID_TABLE: pci::IdTable<Self::IdInfo> = &PCI_TABLE;

    fn probe(pdev: &pci::Device<Core>, _info: &Self::IdInfo) -> impl PinInit<Self, Error> {
        pin_init::pin_init_scope(move || {
            pdev.enable_device_mem()?;
            let bar = KBox::pin_init(pdev.iomap_region_sized::<0x100>(0, c"ch29_fail_mmio"), GFP_KERNEL)?;
            let io = bar.access(pdev.as_ref())?;
            // ANCHOR: past
            // BAR — 0x100 байт; 0x100 — первый байт за его концом.
            let past = io.read32(0x100);
            // ANCHOR_END: past
            dev_info!(pdev, "{past:#x}\n");
            Ok(try_pin_init!(Fail { past }))
        })
    }
}

kernel::module_pci_driver! {
    type: Fail,
    name: "ch29_fail_mmio",
    authors: ["Rust textbook"],
    description: "Must not link",
    license: "GPL",
}
