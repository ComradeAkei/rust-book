//! Функции, которые глава разбирает в ассемблере (разделы 15.1, 15.5 и
//! 15.8): `push` у `Vec` — быстрый путь и вызов роста вне строки, индекс в
//! `VecDeque` — сравнение и вычитание без деления, `push` у
//! `heapless::Vec` — сравнение с константой и никакого распределителя.

#[cfg(feature = "alloc")]
use alloc::{collections::VecDeque, vec::Vec};

#[cfg(feature = "alloc")]
// ANCHOR: push
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch15_push(v: &mut Vec<u32>, x: u32) {
    v.push(x)
}
// ANCHOR_END: push

#[cfg(feature = "alloc")]
// ANCHOR: deque_get
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch15_deque_get(d: &VecDeque<u32>, i: usize) -> Option<u32> {
    d.get(i).copied()
}
// ANCHOR_END: deque_get

// ANCHOR: hpush
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch15_hpush(v: &mut heapless::Vec<u32, 8>, x: u32) -> bool {
    v.push(x).is_ok()
}
// ANCHOR_END: hpush
