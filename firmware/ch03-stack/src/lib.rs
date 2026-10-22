//! Глава 3, раздел 3.5: стек на Cortex-M.
//!
//! На микроконтроллере нет ни MMU, ни страницы-сторожа: стек — это просто
//! верхняя часть ОЗУ, и растёт он вниз, навстречу `.bss` и `.data`. Здесь
//! собраны инструменты для двух опытов:
//!
//! - `stack` закрашивает свободную часть стека известным словом, выполняет
//!   код и ищет нижнюю границу затёртой краски: так видно, сколько стека
//!   код занял на самом деле;
//! - `overflow` занимает стека больше, чем есть, и показывает, что стало со
//!   статической переменной.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch03-stack --bin stack
//! cargo run --release -p ch03-stack --bin overflow
//! cargo run --release -p ch03-stack --bin lockup
//! ```
#![no_std]

use core::hint::black_box;
use cortex_m::register::msp;

// ANCHOR: symbols
// Символы из link.x крейта cortex-m-rt: вершина стека (конец ОЗУ) и его
// нижняя граница (конец `.bss` и `.uninit`). Это адреса, а не переменные:
// значения по ним никто не читает.
unsafe extern "C" {
    static mut _stack_end: u32;
    static _stack_start: u32;
}

/// Взять адрес символа безопасно: память по нему не читается.
pub fn stack_bottom() -> usize {
    &raw mut _stack_end as usize
}

pub fn stack_top() -> usize {
    &raw const _stack_start as usize
}
// ANCHOR_END: symbols

// ANCHOR: paint
const PAINT: u32 = 0xCCCC_CCCC;

/// Закрашивает стек от нижней границы до текущего SP с запасом в 64 байта.
#[inline(never)]
pub fn paint() {
    let sp = msp::read() as usize;
    let mut p = stack_bottom();
    while p < sp - 64 {
        // SAFETY: адреса между концом `.bss` и текущим SP никому не
        // принадлежат: это свободная часть стека, её ещё никто не занял.
        unsafe { (p as *mut u32).write_volatile(PAINT) };
        p += 4;
    }
}

/// Самый низкий адрес, до которого дошёл стек после `paint`.
pub fn lowest_touched() -> usize {
    let mut p = stack_bottom();
    // SAFETY: читаем слова ОЗУ между концом `.bss` и вершиной стека.
    while p < stack_top() && unsafe { (p as *const u32).read_volatile() } == PAINT {
        p += 4;
    }
    p
}

/// Сколько байт стека ниже текущего SP заняла функция `f`.
#[inline(never)]
pub fn stack_used<R>(f: impl FnOnce() -> R) -> (R, usize) {
    paint();
    let sp = msp::read() as usize;
    let r = f();
    (r, sp - lowest_touched())
}
// ANCHOR_END: paint

// ANCHOR: dig
/// Каждый уровень рекурсии кладёт на стек 256 байт. Рекурсия
/// останавливается, когда указатель стека опустился до адреса `stop_at`
/// или ниже.
#[inline(never)]
pub fn dig(level: u32, stop_at: usize) -> u32 {
    let mut buf = [0xEE_u8; 256];
    black_box(&mut buf);
    let deepest = if msp::read() as usize > stop_at { dig(level + 1, stop_at) } else { level };
    black_box(&buf); // буфер нужен и после возврата: кадр живёт всю рекурсию
    deepest
}
// ANCHOR_END: dig
