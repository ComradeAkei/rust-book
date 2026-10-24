// SPDX-License-Identifier: GPL-2.0

//! 29.4: KBox, KVec и KVVec, флаги GFP, отказ выделения, контекст и стек.

use core::hint::black_box;
use core::mem::size_of;
use kernel::{
    alloc::{AllocError, Flags},
    new_spinlock,
    prelude::*,
    sync::SpinLock,
};
use pin_init::stack_pin_init;

module! {
    type: Alloc,
    name: "ch29_alloc",
    authors: ["Rust textbook"],
    description: "Allocations with GFP flags",
    license: "GPL",
    params: {
        under_lock: u32 {
            default: 0,
            description: "1: allocate with GFP_KERNEL while holding a spinlock",
        },
        stack_kib: u32 {
            default: 0,
            description: "Use about this many KiB of kernel stack",
        },
    },
}

struct Point {
    x: i32,
    y: i32,
}

// ANCHOR: basics
fn basics() -> Result {
    pr_info!(
        "size_of: KBox<Point> {}, Option<KBox<Point>> {}, KVec<u8> {}, Flags {}\n",
        size_of::<KBox<Point>>(),
        size_of::<Option<KBox<Point>>>(),
        size_of::<KVec<u8>>(),
        size_of::<Flags>(),
    );
    let p = KBox::new(Point { x: 3, y: 4 }, GFP_KERNEL)?;
    pr_info!("KBox: ({}, {})\n", p.x, p.y);

    let mut v = KVec::new();
    for i in 1..=5 {
        // Каждый push может выделять память — и каждый получает флаги.
        v.push(i * i, GFP_KERNEL)?;
    }
    pr_info!("KVec: {:?}, ёмкость {}\n", v, v.capacity());
    Ok(())
}
// ANCHOR_END: basics

// ANCHOR: big
fn big() {
    let huge: Result<KVec<u8>, AllocError> = KVec::with_capacity(1 << 40, GFP_KERNEL | __GFP_NOWARN);
    pr_info!("KVec на 1 ТиБ: {:?}\n", huge.err());

    // kmalloc отдаёт непрерывные физические страницы — 64 МиБ ему не по силам;
    // kvmalloc (KVVec) после отказа kmalloc берёт память у vmalloc.
    let k = KVec::<u8>::with_capacity(64 << 20, GFP_KERNEL | __GFP_NOWARN);
    let kv = KVVec::<u8>::with_capacity(64 << 20, GFP_KERNEL);
    pr_info!("64 МиБ: KVec {:?}, KVVec {:?}\n", k.as_ref().err(), kv.as_ref().err());
    // Адрес печатаем числом: `{:p}` в ядре 7.0 показывает не указатель,
    // а адрес временной ссылки на него — см. строку ниже и раздел 29.4.
    if let Ok(kv) = &kv {
        pr_info!("KVVec по адресу {:#x}\n", kv.as_ptr() as usize);
        pr_info!("тот же указатель через {{:p}}: {:p}\n", kv.as_ptr());
    }
    let small = KVVec::<u8>::with_capacity(4096, GFP_KERNEL);
    if let Ok(small) = &small {
        pr_info!("KVVec на 4 КиБ по адресу {:#x}\n", small.as_ptr() as usize);
    }
}
// ANCHOR_END: big

// ANCHOR: atomic
/// Выделение под спин-блокировкой: держатель спин-блокировки не имеет
/// права спать, а GFP_KERNEL разрешает аллокатору заснуть.
fn under(lock: &SpinLock<u32>, flags: Flags) -> Result {
    let mut guard = lock.lock();
    *guard += 1;
    let buf = KBox::new([0u8; 64], flags)?;
    black_box(&buf);
    Ok(())
}
// ANCHOR_END: atomic

// ANCHOR: stack
/// Около килобайта стека на кадр: `black_box` не даёт компилятору
/// убрать массив или превратить рекурсию в цикл.
#[inline(never)]
fn burn(depth: u32) -> u32 {
    let frame = black_box([depth as u8; 1024]);
    if depth == 0 { u32::from(frame[0]) } else { burn(depth - 1) + u32::from(frame[1023]) }
}
// ANCHOR_END: stack

struct Alloc;

impl kernel::Module for Alloc {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        basics()?;
        big();
        stack_pin_init!(let lock = new_spinlock!(0u32, "ch29_alloc"));
        under(&lock, GFP_ATOMIC)?;
        pr_info!("GFP_ATOMIC под спин-блокировкой: ok\n");
        if *module_parameters::under_lock.value() == 1 {
            under(&lock, GFP_KERNEL)?;
            pr_info!("GFP_KERNEL под спин-блокировкой: вернулось\n");
        }
        let kib = *module_parameters::stack_kib.value();
        if kib > 0 {
            pr_info!("стек: рекурсия на {kib} КиБ\n");
            pr_info!("стек: сумма {}\n", burn(kib));
        }
        Ok(Alloc)
    }
}
