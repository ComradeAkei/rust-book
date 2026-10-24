// SPDX-License-Identifier: GPL-2.0

//! Задача 29.B: модуль с параметром `n` (по умолчанию 10) печатает при
//! загрузке `fib(n) = …`; если F(n) не помещается в u64 — сообщение об
//! ошибке и отказ загрузки с EOVERFLOW. Заготовка уже собирается и
//! грузится: начните с того, чтобы увидеть её строку в журнале.

use kernel::prelude::*;

module! {
    type: Fib,
    name: "ch29_b",
    authors: ["Rust textbook"],
    description: "Task 29.B: Fibonacci number from a parameter",
    license: "GPL",
}

struct Fib;

impl kernel::Module for Fib {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("заготовка задачи 29.B: параметра n ещё нет\n");
        Ok(Fib)
    }
}
