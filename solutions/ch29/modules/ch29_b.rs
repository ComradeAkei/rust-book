// SPDX-License-Identifier: GPL-2.0

//! Задача 29.B: модуль с параметром `n` печатает n-е число Фибоначчи.

use kernel::prelude::*;

module! {
    type: Fib,
    name: "ch29_b",
    authors: ["Rust textbook"],
    description: "Task 29.B: Fibonacci number from a parameter",
    license: "GPL",
    params: {
        n: u32 {
            default: 10,
            description: "Index of the Fibonacci number",
        },
    },
}

// ANCHOR: fib
/// F(n) или `None`, если оно не помещается в u64. Складывать надо до
/// F(n), а не до F(n+1): F(93) помещается, а F(94) уже нет.
fn fib(n: u32) -> Option<u64> {
    if n == 0 {
        return Some(0);
    }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 1..n {
        let next = a.checked_add(b)?;
        a = b;
        b = next;
    }
    Some(b)
}
// ANCHOR_END: fib

struct Fib;

impl kernel::Module for Fib {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let n = *module_parameters::n.value();
        match fib(n) {
            Some(v) => pr_info!("fib({n}) = {v}\n"),
            None => {
                pr_err!("fib({n}) не помещается в u64\n");
                return Err(EOVERFLOW);
            }
        }
        Ok(Fib)
    }
}

impl Drop for Fib {
    fn drop(&mut self) {
        pr_info!("выгружен\n");
    }
}
