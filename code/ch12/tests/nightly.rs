//! Раздел 12.2: то, что есть только в nightly. Трейты `Fn*` нельзя
//! реализовать на stable, но с `unboxed_closures` и `fn_traits` видно, что
//! компилятор строит для замыкания `move |x| x * k`: структуру с полем `k`
//! и три реализации, где аргументы приходят одним кортежем по ABI
//! `"rust-call"`.
//!
//! Запуск: `cargo +nightly test -p ch12 --features nightly --test nightly`.
//! Без возможности `nightly` файл пуст и собирается на stable.
#![cfg(feature = "nightly")]
#![cfg_attr(feature = "nightly", feature(unboxed_closures, fn_traits))]

// ANCHOR: manual
/// Модель `move |x: u32| x * k`.
struct Scale {
    k: u32,
}

impl FnOnce<(u32,)> for Scale {
    type Output = u32;
    extern "rust-call" fn call_once(self, args: (u32,)) -> u32 {
        self.call(args)
    }
}

impl FnMut<(u32,)> for Scale {
    extern "rust-call" fn call_mut(&mut self, args: (u32,)) -> u32 {
        self.call(args)
    }
}

impl Fn<(u32,)> for Scale {
    extern "rust-call" fn call(&self, (x,): (u32,)) -> u32 {
        x * self.k
    }
}

/// `Fn(u32) -> u32` — сокращение для `Fn<(u32,), Output = u32>`.
fn apply<F: Fn<(u32,), Output = u32>>(f: F, x: u32) -> u32 {
    f(x)
}
// ANCHOR_END: manual

#[test]
fn manual_closure() {
    let s = Scale { k: 3 };
    assert_eq!(s(2), 6);
    assert_eq!(apply(Scale { k: 2 }, 5), 10);
    assert_eq!(apply(|x| x + 1, 5), 6);
    let v: Vec<u32> = [1, 2, 3].into_iter().map(Scale { k: 10 }).collect();
    assert_eq!(v, [10, 20, 30]);
    assert_eq!(size_of::<Scale>(), size_of_val(&move |x: u32| x * s.k));
}
