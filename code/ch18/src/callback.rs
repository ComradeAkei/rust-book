//! Раздел 18.5: обратные вызовы. C-функция принимает указатель на функцию
//! фиксированного типа и `void *` для контекста. Замыкание Rust так передать
//! нельзя — у него нет C-ABI и оно несёт захваты. Мост — переходник
//! (trampoline): свободная `extern "C"`-функция нужного типа, которая достаёт
//! замыкание из `ctx` и вызывает его. И правило: паника не должна дойти до
//! границы C — до Rust 1.81 это было неопределённое поведение, теперь это
//! аварийное завершение процесса, — поэтому её ловят `catch_unwind`.

use core::ffi::{c_int, c_void};
use std::panic::{self, AssertUnwindSafe};

unsafe extern "C" {
    fn c_apply(f: extern "C" fn(c_int, *mut c_void) -> c_int, x: c_int, ctx: *mut c_void) -> c_int;
    fn c_fold(
        arr: *const c_int,
        n: usize,
        f: extern "C" fn(c_int, c_int, *mut c_void) -> c_int,
        init: c_int,
        ctx: *mut c_void,
    ) -> c_int;
}

// ANCHOR: trampoline
/// Переходник для `apply`: C-функция фиксированного типа. Через `ctx`
/// получает замыкание и вызывает его. Паника наружу (в C) уйти не должна,
/// поэтому её ловит `catch_unwind`, а вместо значения уходит -1.
extern "C" fn apply_trampoline<F: FnMut(i32) -> i32>(x: c_int, ctx: *mut c_void) -> c_int {
    // SAFETY: ctx — указатель на F, переданный в apply ниже; замыкание живо
    // всё время синхронного вызова c_apply.
    let f = unsafe { &mut *(ctx as *mut F) };
    panic::catch_unwind(AssertUnwindSafe(|| f(x))).unwrap_or(-1)
}

/// Передать замыкание в C как обратный вызов `int f(int, void*)`.
pub fn apply<F: FnMut(i32) -> i32>(x: i32, mut f: F) -> i32 {
    let ctx = (&mut f as *mut F).cast::<c_void>();
    // SAFETY: apply_trampoline::<F> имеет требуемый тип; ctx указывает на f,
    // живую до конца вызова; c_apply вызывает переходник синхронно.
    unsafe { c_apply(apply_trampoline::<F>, x, ctx) }
}
// ANCHOR_END: trampoline

// ANCHOR: fold
/// Переходник для `fold`: вызывается по разу на элемент, накапливая через
/// замыкание в `ctx`. Показывает контекст-состояние, живущее между вызовами.
extern "C" fn fold_trampoline<F: FnMut(i32, i32) -> i32>(
    acc: c_int,
    x: c_int,
    ctx: *mut c_void,
) -> c_int {
    // SAFETY: ctx — указатель на F из fold ниже, живой весь обход c_fold.
    let f = unsafe { &mut *(ctx as *mut F) };
    panic::catch_unwind(AssertUnwindSafe(|| f(acc, x))).unwrap_or(acc)
}

/// Свернуть срез в C-цикле, передав замыкание-аккумулятор.
pub fn fold<F: FnMut(i32, i32) -> i32>(data: &[i32], init: i32, mut f: F) -> i32 {
    let ctx = (&mut f as *mut F).cast::<c_void>();
    // SAFETY: fold_trampoline::<F> нужного типа; data даёт указатель и длину;
    // ctx указывает на f, живую весь вызов; c_fold идёт по n элементам.
    unsafe { c_fold(data.as_ptr(), data.len(), fold_trampoline::<F>, init, ctx) }
}
// ANCHOR_END: fold

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn apply_runs_closure() {
        let factor = 3;
        assert_eq!(apply(10, |x| x * factor), 30);
    }

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn panic_does_not_cross_ffi() {
        let prev = panic::take_hook();
        panic::set_hook(Box::new(|_| {})); // не печатать панику в тесте
        let r = apply(1, |_| panic!("паника внутри обратного вызова"));
        panic::set_hook(prev);
        assert_eq!(r, -1); // поймана, граница C не пересечена
    }

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn fold_accumulates_through_ctx() {
        let data = [1, 2, 3, 4];
        let sum = fold(&data, 0, |acc, x| acc + x);
        assert_eq!(sum, 10);
    }
}
