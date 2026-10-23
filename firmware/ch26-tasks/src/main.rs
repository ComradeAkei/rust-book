//! Глава 26, задача D, на Cortex-M: функции раздела 26.5 копируют и
//! обнуляют память, а вызовы, которые вставил компилятор, приходят в
//! `memcpy`, `memmove` и `memset` из `ch26-solutions`, а не в слабые
//! определения `compiler_builtins`. Доказательство — счётчики вызовов:
//! их увеличивают только наши функции (возможность `count`).
//!
//! ```text
//! cd firmware && cargo run --release -p ch26-tasks
//! ```
//!
//! Работает на всех трёх ядрах: на ARMv6-M счётчики — `load` и `store`
//! без `fetch_add`.
#![no_std]
#![no_main]

use ch26::copies::{Block, ch26_clear, ch26_copy_frame, ch26_fill, ch26_store};
use ch26_solutions::d_mem::calls::{MEMCPY, MEMMOVE, MEMSET};
use core::hint::black_box;
use core::sync::atomic::Ordering::Relaxed;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

// ANCHOR: calls
/// Показания трёх счётчиков.
#[derive(Clone, Copy, PartialEq)]
struct Calls {
    memcpy: u32,
    memmove: u32,
    memset: u32,
}

fn calls() -> Calls {
    Calls {
        memcpy: MEMCPY.load(Relaxed),
        memmove: MEMMOVE.load(Relaxed),
        memset: MEMSET.load(Relaxed),
    }
}

/// Сколько раз `f` позвала каждую из функций и верен ли её результат.
/// Между двумя показаниями нет печати: `hprintln` и сам может копировать.
fn measure(f: impl FnOnce() -> bool) -> (Calls, bool) {
    let a = calls();
    let ok = f();
    let b = calls();
    let d = Calls {
        memcpy: b.memcpy.wrapping_sub(a.memcpy),
        memmove: b.memmove.wrapping_sub(a.memmove),
        memset: b.memset.wrapping_sub(a.memset),
    };
    (d, ok)
}
// ANCHOR_END: calls

const fn only(memcpy: u32, memmove: u32, memset: u32) -> Calls {
    Calls { memcpy, memmove, memset }
}

// ANCHOR: main
#[entry]
fn main() -> ! {
    // Reset из cortex-m-rt копирует .data и обнуляет .bss циклами на
    // ассемблере: до main ни одна из трёх функций не вызывалась.
    let boot = calls();
    hprintln!("до main: memcpy {}, memmove {}, memset {}", boot.memcpy, boot.memmove, boot.memset);

    let src: [u8; 40] = core::array::from_fn(|i| i as u8 * 3);
    let mut frame = [0u8; 64];
    let mut slot = Block { words: [7; 64] };
    let block = Block { words: core::array::from_fn(|i| i as u32) };
    let mut buf = [1u8; 48];
    let mut ring: [u8; 32] = core::array::from_fn(|i| i as u8);

    let cases: [(&str, Calls, (Calls, bool)); 5] = [
        (
            "ch26_copy_frame",
            only(1, 0, 0),
            measure(|| {
                ch26_copy_frame(&mut frame, black_box(&src)) == 40
                    && frame[..40] == src
                    && frame[40] == 0
            }),
        ),
        (
            "ch26_store",
            only(1, 0, 0),
            measure(|| {
                ch26_store(&mut slot, black_box(&block));
                slot.words == block.words
            }),
        ),
        (
            "ch26_clear",
            only(0, 0, 1),
            measure(|| {
                ch26_clear(black_box(&mut slot));
                slot.words == [0; 64]
            }),
        ),
        (
            "ch26_fill",
            only(0, 0, 1),
            measure(|| {
                ch26_fill(black_box(&mut buf[5..45]), 0xAA);
                buf[4] == 1 && buf[5..45].iter().all(|&b| b == 0xAA) && buf[45] == 1
            }),
        ),
        (
            "ptr::copy",
            only(0, 1, 0),
            measure(|| {
                // Сдвиг на байт вправо внутри одного массива: области
                // перекрываются, нужна memmove. Длина — из black_box, иначе
                // LLVM раскрыл бы копию на месте.
                let p = ring.as_mut_ptr();
                // SAFETY: обе области — внутри ring.
                unsafe { core::ptr::copy(p, p.add(1), black_box(31)) };
                ring[0] == 0 && (1..32).all(|i| ring[i] == i as u8 - 1)
            }),
        ),
    ];

    let mut good = 0;
    for (name, want, (got, ok)) in cases {
        hprintln!(
            "{:16} memcpy {}  memmove {}  memset {}  {}",
            name,
            got.memcpy,
            got.memmove,
            got.memset,
            if ok { "результат верен" } else { "РЕЗУЛЬТАТ НЕВЕРЕН" }
        );
        if ok && got == want {
            good += 1;
        }
    }
    hprintln!("вызовы пришли в наши функции: {} из {}", good, cases.len());
    debug::exit(if good == cases.len() { debug::EXIT_SUCCESS } else { debug::EXIT_FAILURE });
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
