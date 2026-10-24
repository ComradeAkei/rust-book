// SPDX-License-Identifier: GPL-2.0

//! 29.8: драйвер учебного PCI-устройства QEMU `edu` (`-device edu`):
//! регистры в BAR0 через MMIO, вычисление факториала и прерывание MSI.
//! Описание устройства — docs/specs/edu.rst в исходниках QEMU.

use kernel::{
    device::{Bound, Core, Device},
    devres::Devres,
    io::Io,
    irq::{self, IrqReturn},
    pci,
    prelude::*,
    sync::{
        aref::ARef,
        atomic::{Atomic, Relaxed},
        Arc,
    },
    time::{delay::fsleep, Delta},
};

// ANCHOR: regs
/// Регистры edu: смещения в BAR0.
struct Regs;

impl Regs {
    /// Только чтение: 0xRRrr00ed, RR.rr — версия.
    const ID: usize = 0x00;
    /// Запись x — чтение вернёт !x: проверка, что устройство живо.
    const LIVENESS: usize = 0x04;
    /// Запись n запускает n!, результат читается отсюда же.
    const FACTORIAL: usize = 0x08;
    /// Бит 0 — идёт вычисление, бит 7 — прерывание по окончании.
    const STATUS: usize = 0x20;
    /// Причины прерывания: 1 — факториал, остальное — из IRQ_RAISE.
    const IRQ_STATUS: usize = 0x24;
    const IRQ_RAISE: usize = 0x60;
    const IRQ_ACK: usize = 0x64;
    const END: usize = 0x100;
}

const STATUS_COMPUTING: u32 = 0x01;
const STATUS_IRQ_ON_DONE: u32 = 0x80;

/// Проверки границ — при компиляции: смещение плюс размер доступа
/// должны уложиться в END.
type Bar0 = pci::Bar<{ Regs::END }>;
// ANCHOR_END: regs

// ANCHOR: vendor
// SAFETY: `pci::Vendor` — `#[repr(transparent)]` над u16, а конструктор
// из числа в 7.0 закрыт (`pub(super)`): в таблице производителей ядра
// нет 0x1234, которым QEMU помечает свои учебные устройства.
const QEMU_VENDOR: pci::Vendor = unsafe { core::mem::transmute::<u16, pci::Vendor>(0x1234) };
const EDU_DEVICE: u32 = 0x11e8;
// ANCHOR_END: vendor

kernel::pci_device_table!(
    PCI_TABLE,
    MODULE_PCI_TABLE,
    <Edu as pci::Driver>::IdInfo,
    [(pci::DeviceId::from_id(QEMU_VENDOR, EDU_DEVICE), ())]
);

// ANCHOR: handler
/// Обработчик прерывания: выполняется в контексте прерывания — спать,
/// брать мьютекс и выделять память с GFP_KERNEL здесь нельзя.
struct EduIrq {
    bar: Arc<Devres<Bar0>>,
    count: Atomic<u32>,
    causes: Atomic<u32>,
}

impl irq::Handler for EduIrq {
    fn handle(&self, dev: &Device<Bound>) -> IrqReturn {
        let Ok(bar) = self.bar.access(dev) else {
            return IrqReturn::None;
        };
        let status = bar.read32(Regs::IRQ_STATUS);
        if status == 0 {
            return IrqReturn::None;
        }
        bar.write32(status, Regs::IRQ_ACK);
        self.causes.store(self.causes.load(Relaxed) | status, Relaxed);
        self.count.add(1, Relaxed);
        IrqReturn::Handled
    }
}
// ANCHOR_END: handler

#[pin_data]
struct Edu {
    pdev: ARef<pci::Device>,
    bar: Arc<Devres<Bar0>>,
    #[pin]
    irq: irq::Registration<EduIrq>,
}

/// Ждёт, пока счётчик прерываний не станет больше `seen`, не дольше 1 с.
fn wait_irq(handler: &EduIrq, seen: u32) -> Result<u32> {
    for _ in 0..1000 {
        let n = handler.count.load(Relaxed);
        if n > seen {
            return Ok(n);
        }
        fsleep(Delta::from_millis(1));
    }
    Err(ETIMEDOUT)
}

// ANCHOR: probe
impl pci::Driver for Edu {
    type IdInfo = ();
    const ID_TABLE: pci::IdTable<Self::IdInfo> = &PCI_TABLE;

    fn probe(pdev: &pci::Device<Core>, _info: &Self::IdInfo) -> impl PinInit<Self, Error> {
        pin_init::pin_init_scope(move || {
            pdev.enable_device_mem()?;
            pdev.set_master();
            let bar = Arc::pin_init(pdev.iomap_region_sized::<{ Regs::END }>(0, c"ch29_edu"), GFP_KERNEL)?;
            {
                let io = bar.access(pdev.as_ref())?;
                let id = io.read32(Regs::ID);
                io.write32(0x1234_5678, Regs::LIVENESS);
                let live = io.read32(Regs::LIVENESS);
                dev_info!(pdev, "edu {}.{}, проверка: !0x12345678 = {live:#010x}\n", id >> 24, (id >> 16) & 0xff);
            }
            let msi = pci::IrqTypes::default().with(pci::IrqType::Msi);
            let vectors = pdev.alloc_irq_vectors(1, 1, msi)?;
            let handler = EduIrq { bar: bar.clone(), count: Atomic::new(0), causes: Atomic::new(0) };
            Ok(try_pin_init!(Self {
                irq <- pdev.request_irq(*vectors.start(), irq::Flags::TRIGGER_NONE, c"ch29_edu", Ok::<_, Error>(handler)),
                _: {
                    let io = bar.access(pdev.as_ref())?;
                    let h = irq.handler();
                    // Факториал с прерыванием по окончании.
                    io.write32(STATUS_IRQ_ON_DONE, Regs::STATUS);
                    io.write32(10, Regs::FACTORIAL);
                    let n = wait_irq(h, 0)?;
                    let busy = io.read32(Regs::STATUS) & STATUS_COMPUTING;
                    dev_info!(pdev, "10! = {} (прерываний: {n}, занято: {busy})\n", io.read32(Regs::FACTORIAL));
                    // Прерывание по просьбе драйвера: причина 0x100.
                    io.write32(0x100, Regs::IRQ_RAISE);
                    let n = wait_irq(h, n)?;
                    dev_info!(pdev, "IRQ_RAISE: прерываний {n}, причины {:#x}\n", h.causes.load(Relaxed));
                },
                bar,
                pdev: pdev.into(),
            }))
        })
    }

    fn unbind(pdev: &pci::Device<Core>, this: Pin<&Self>) {
        dev_info!(pdev, "отключение: всего прерываний {}\n", this.irq.handler().count.load(Relaxed));
    }
}
// ANCHOR_END: probe

kernel::module_pci_driver! {
    type: Edu,
    name: "ch29_edu",
    authors: ["Rust textbook"],
    description: "Driver for the QEMU edu PCI device",
    license: "GPL",
}
