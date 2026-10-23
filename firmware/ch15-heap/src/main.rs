//! Глава 15 на Cortex-M0, M3 и M4F: куча в 4 КиБ на статическом массиве
//! через embedded-alloc, рост `Vec` без `realloc` на месте, фрагментация и
//! `try_reserve` вместо аварийной остановки, коллекции heapless, которым
//! куча не нужна. Последний шаг нарочно просит больше, чем есть: `vec!` не
//! умеет вернуть ошибку, и прошивка завершается паникой.
//!
//! ```text
//! cd firmware && cargo run --release -p ch15-heap
//! ```
#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use embedded_alloc::LlffHeap as Heap;

// ANCHOR: heap
#[global_allocator]
static HEAP: Heap = Heap::empty();

const HEAP_SIZE: usize = 4096;
static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

fn init_heap() {
    // SAFETY: вызывается один раз, до первого выделения; к массиву больше
    // никто не обращается.
    unsafe { HEAP.init(&raw mut HEAP_MEM as usize, HEAP_SIZE) }
}
// ANCHOR_END: heap

// ANCHOR: growth
/// Рост `Vec<u32>` до `n` элементов: сколько раз буфер переехал. У
/// распределителя нет своего `realloc`, и `GlobalAlloc::realloc` по
/// умолчанию выделяет новый блок, копирует и освобождает старый.
fn moves(n: usize) -> (usize, usize) {
    let mut v: Vec<u32> = Vec::new();
    let (mut ptr, mut moved) = (v.as_ptr(), 0);
    for i in 0..n as u32 {
        v.push(i);
        if v.as_ptr() != ptr {
            moved += 1;
            ptr = v.as_ptr();
        }
    }
    (v.capacity(), moved)
}
// ANCHOR_END: growth

#[entry]
fn main() -> ! {
    init_heap();
    hprintln!("куча {} байт, занято {}, свободно {}", HEAP_SIZE, HEAP.used(), HEAP.free());

    let (cap, moved) = moves(100);
    hprintln!("Vec<u32> до 100 элементов: ёмкость {}, переездов {}", cap, moved);

    // ANCHOR: fragment
    // Восемь блоков по 480 байт, затем каждый второй освобождён.
    let mut blocks: Vec<Option<Box<[u8; 480]>>> = Vec::with_capacity(8);
    for i in 0..8 {
        blocks.push(Some(Box::new([i; 480])));
    }
    hprintln!("8 блоков по 480: занято {}, свободно {}", HEAP.used(), HEAP.free());
    for b in blocks.iter_mut().step_by(2) {
        *b = None;
    }
    hprintln!("каждый второй освобождён: занято {}, свободно {}", HEAP.used(), HEAP.free());
    let mut buf: Vec<u8> = Vec::new();
    match buf.try_reserve_exact(800) {
        Ok(()) => hprintln!("800 байт одним куском: есть"),
        Err(e) => hprintln!("800 байт одним куском: {}", e),
    }
    // Соседние свободные участки сливаются в один.
    blocks[1] = None;
    match buf.try_reserve_exact(800) {
        Ok(()) => hprintln!("после освобождения соседа: 800 байт есть"),
        Err(e) => hprintln!("после освобождения соседа: {}", e),
    }
    // ANCHOR_END: fragment
    drop(buf);
    drop(blocks);

    // ANCHOR: heapless
    let before = HEAP.used();
    let m = ch15::nostd::last_values(&[(1, 10), (2, 20), (1, 11)]).unwrap();
    let ninth: [(u16, i32); 9] = core::array::from_fn(|i| (i as u16 + 1, 0));
    let full = ch15::nostd::last_values(&ninth).unwrap_err();
    let line = ch15::nostd::line(7, -42).unwrap();
    let mut q = heapless::Deque::new();
    let queued = (0..5).filter(|&i| ch15::nostd::enqueue(&mut q, [i; 18]).is_ok()).count();
    hprintln!(
        "heapless: датчиков {}, не поместился {}, строка «{}», в очереди {} из 5, куча не тронута: {}",
        m.len(),
        full,
        line,
        queued,
        HEAP.used() == before
    );
    let [v, v8, s, d, map] = ch15::nostd::sizes();
    hprintln!(
        "размеры: Vec<u8, 16> {}, Vec<u8, 16, u8> {}, String<16> {}, Deque<[u8; 18], 4> {}, FnvIndexMap<u16, i32, 8> {}, alloc::vec::Vec<u8> {}",
        v,
        v8,
        s,
        d,
        map,
        size_of::<Vec<u8>>()
    );
    // ANCHOR_END: heapless

    // ANCHOR: oom
    // vec! не возвращает ошибку: при нехватке памяти — handle_alloc_error,
    // а в no_std он паникует.
    let big = alloc::vec![0u8; 5000];
    hprintln!("не дойдёт: {}", big.len());
    // ANCHOR_END: oom
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    hprintln!("паника: {}", info.message());
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
