// Задача D: тот же код memcpy и memset, что в src/d_mem.rs, в крейте
// без #![no_builtins]. Тест tests/naive.rs собирает его под Cortex-M3.
#![no_std]

#[allow(dead_code)]
#[path = "../src/d_mem.rs"]
mod d_mem;
