// SPDX-License-Identifier: GPL-2.0

//! 29.8: допустимое смещение, которое компилятор не видит. `read32`
//! проверяет границы только при компиляции, и если смещение — значение
//! времени выполнения, вызов `rust_build_error` остаётся в коде, хотя
//! 0x00 < 0x100: модуль не соберёт modpost. Для таких смещений —
//! `try_read32`, с проверкой при выполнении.

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
    id: u32,
}

impl pci::Driver for Fail {
    type IdInfo = ();
    const ID_TABLE: pci::IdTable<Self::IdInfo> = &PCI_TABLE;

    fn probe(pdev: &pci::Device<Core>, _info: &Self::IdInfo) -> impl PinInit<Self, Error> {
        pin_init::pin_init_scope(move || {
            pdev.enable_device_mem()?;
            let bar = KBox::pin_init(pdev.iomap_region_sized::<0x100>(0, c"ch29_fail_mmio_var"), GFP_KERNEL)?;
            let io = bar.access(pdev.as_ref())?;
            // ANCHOR: var
            // Смещение 0 — регистр ID, но значение приходит при выполнении.
            let offset = core::hint::black_box(0usize);
            let id = io.read32(offset);
            // ANCHOR_END: var
            dev_info!(pdev, "{id:#x}\n");
            Ok(try_pin_init!(Fail { id }))
        })
    }
}

kernel::module_pci_driver! {
    type: Fail,
    name: "ch29_fail_mmio_var",
    authors: ["Rust textbook"],
    description: "Must not link",
    license: "GPL",
}
