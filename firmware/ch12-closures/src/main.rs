//! Глава 12 на Cortex-M0, M3 и M4F: замыкание занимает столько, сколько
//! захватило; замыкания без захвата лежат в таблице обработчиков во flash
//! как обычные указатели на функции, с единицей в младшем бите (состояние
//! Thumb); разбор кадра комбинаторами даёт то же, что `decode`.
//!
//! ```text
//! cd firmware && cargo run --release -p ch12-closures
//! ```
#![no_std]
#![no_main]

use ch12::fnptr::{HANDLERS, ch12_dispatch};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use telemetry_frame::{Kind, decode, encode, parse};

fn size<F>(_: &F) -> usize {
    size_of::<F>()
}

// ANCHOR: main
#[entry]
fn main() -> ! {
    let x = core::hint::black_box(5i32);
    let mut n = 0u64;
    let by_ref = || x + 1;
    let by_value = move || x + 1;
    let mut counter = || n += 1;
    hprintln!(
        "|| x + 1: {} байта, move || x + 1: {} байта, || n += 1 (u64): {} байта",
        size(&by_ref),
        size(&by_value),
        size(&counter)
    );
    counter();
    hprintln!(
        "&mut dyn FnMut(): {} байт, fn(u8) -> u8: {} байта",
        size_of::<&mut dyn FnMut()>(),
        size_of::<fn(u8) -> u8>()
    );

    let table = HANDLERS.as_ptr() as usize;
    hprintln!(
        "HANDLERS: {:#010x} {}",
        table,
        if table < 0x2000_0000 { "во flash" } else { "в RAM" }
    );
    let mut odd = 0;
    for (i, f) in HANDLERS.iter().enumerate() {
        let addr = *f as usize;
        odd += addr & 1;
        hprintln!("  [{}] {:#010x} -> {:#04x}", i, addr, ch12_dispatch(i, 0xa1));
    }
    hprintln!("адресов с битом Thumb: {} из {}", odd, HANDLERS.len());

    let mut stream = [0u8; 32];
    let a = encode(Kind::Ack, 1, &[], &mut stream).unwrap();
    let b = encode(Kind::Event, 2, &[7, 7], &mut stream[a..]).unwrap();
    stream[a + b - 1] ^= 0x10;
    let mut same = 0;
    let mut at = 0;
    for _ in 0..3 {
        let rest = &stream[at..];
        let (old, new) = (decode(rest), parse::decode(rest));
        same += usize::from(old == new);
        hprintln!("смещение {}: {:?}", at, new.map(|f| (f.header.kind, f.header.seq)));
        at += new.map_or(1, |f| f.wire_len());
    }
    hprintln!("parse::decode совпал с decode: {} из 3", same);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
