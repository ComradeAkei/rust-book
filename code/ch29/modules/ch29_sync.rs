// SPDX-License-Identifier: GPL-2.0

//! 29.6: общий счётчик для четырёх работ в очереди ядра — под
//! спин-блокировкой и атомарный. Без синхронизации Rust такой код не
//! соберёт: см. modules-fail/racy.

use kernel::{
    cpu::CpuId,
    new_spinlock,
    prelude::*,
    sync::{
        atomic::{Atomic, Full, Relaxed},
        Arc, Completion, SpinLock,
    },
    workqueue,
};

module! {
    type: SyncDemo,
    name: "ch29_sync",
    authors: ["Rust textbook"],
    description: "Shared counters in work items",
    license: "GPL",
}

const WORKERS: u32 = 4;
const ROUNDS: u64 = 100_000;

// ANCHOR: shared
#[pin_data]
struct Shared {
    #[pin]
    locked: SpinLock<u64>,
    atomic: Atomic<u64>,
    finished: Atomic<u32>,
    #[pin]
    done: Completion,
}
// ANCHOR_END: shared

// ANCHOR: run
fn run() -> Result {
    let shared = Arc::pin_init(
        pin_init!(Shared {
            locked <- new_spinlock!(0, "ch29_sync::locked"),
            atomic: Atomic::new(0),
            finished: Atomic::new(0),
            done <- Completion::new(),
        }),
        GFP_KERNEL,
    )?;
    for w in 0..WORKERS {
        let s = shared.clone();
        workqueue::system_unbound().try_spawn(GFP_KERNEL, move || {
            let cpu = CpuId::current().as_u32();
            for _ in 0..ROUNDS {
                *s.locked.lock() += 1;
                s.atomic.add(1, Relaxed);
            }
            pr_info!("работа {w}: CPU {cpu}\n");
            if s.finished.fetch_add(1, Full) + 1 == WORKERS {
                s.done.complete_all();
            }
        })?;
    }
    shared.done.wait_for_completion();
    let expected = u64::from(WORKERS) * ROUNDS;
    pr_info!(
        "под SpinLock: {}, Atomic: {}, ожидалось {expected}\n",
        *shared.locked.lock(),
        shared.atomic.load(Relaxed)
    );
    Ok(())
}
// ANCHOR_END: run

struct SyncDemo;

impl kernel::Module for SyncDemo {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        run()?;
        Ok(SyncDemo)
    }
}
