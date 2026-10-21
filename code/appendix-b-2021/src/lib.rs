//! Приложение B: исходники из `code/appendix-b/src/shared/`, собранные в
//! edition 2021. Тесты фиксируют старое поведение, doc-тесты — код,
//! который в 2021 не компилировался, а в 2024 компилируется.
//!
//! Временное значение в хвостовом выражении жило дольше локальных
//! переменных:
//!
//! ```compile_fail,E0597
//! use std::cell::RefCell;
//! fn len() -> usize {
//!     let c = RefCell::new(vec![1, 2]);
//!     c.borrow().len()
//! }
//! ```
//!
//! `impl Trait` не захватывал время жизни без явного указания:
//!
//! ```compile_fail,E0700
//! fn bytes<'a>(v: &'a [u8]) -> impl Iterator<Item = u8> {
//!     v.iter().copied()
//! }
//! ```
//!
//! `Future` не входил в прелюдию:
//!
//! ```compile_fail,E0405
//! fn is_future<F: Future>(_: &F) -> bool {
//!     true
//! }
//! ```

/* ANCHOR: tail_2021
fn len() -> usize {
    let c = RefCell::new(vec![1, 2]);
    c.borrow().len()
}
// edition 2021:
// error[E0597]: `c` does not live long enough
// note: the temporary is part of an expression at the end of a block;
//       consider forcing this temporary to be dropped sooner, before the block's local variables are dropped
ANCHOR_END: tail_2021 */

/* ANCHOR: rpit_2021
fn bytes<'a>(v: &'a [u8]) -> impl Iterator<Item = u8> {
    v.iter().copied()
}
// edition 2021:
// error[E0700]: hidden type for `impl Iterator<Item = u8>` captures lifetime that does not appear in bounds
// help: add a `use<...>` bound to explicitly capture `'a`
ANCHOR_END: rpit_2021 */

// `into_iter()` у `Box<[T]>` в 2021 выдаёт ссылки, и компилятор об этом
// предупреждает; здесь это поведение и проверяется.
#[allow(boxed_slice_into_iter)]
#[path = "../../appendix-b/src/shared/misc.rs"]
pub mod misc;
#[path = "../../appendix-b/src/shared/temporaries.rs"]
pub mod temporaries;

// ANCHOR: binding_2021
/// В 2021 `mut` в образце сбрасывал режим привязки: `x` — копия `u32`, а
/// не `&u32`, хотя `opt` — ссылка.
pub fn bumped(opt: &Option<u32>) -> Option<u32> {
    if let Some(mut x) = opt {
        x += 1;
        Some(x)
    } else {
        None
    }
}
// ANCHOR_END: binding_2021

#[cfg(test)]
mod tests {
    use super::misc::*;
    use super::temporaries::*;
    use std::sync::Mutex;

    // ANCHOR: tests_2021
    #[test]
    fn if_let_temporary_lives_through_else() {
        let log = Log::default();
        if_let_order(&log);
        assert_eq!(*log.borrow(), ["else", "drop scrutinee"]);
    }

    #[test]
    fn else_branch_cannot_relock() {
        assert!(!relock_in_else(&Mutex::new(None)));
    }

    #[test]
    fn tail_temporary_outlives_locals() {
        let log = Log::default();
        tail_order(&log);
        assert_eq!(*log.borrow(), ["drop local", "drop temporary"]);
    }
    // ANCHOR_END: tests_2021

    #[test]
    fn boxed_slice_yields_references() {
        let b: Box<[u32]> = vec![7, 8].into_boxed_slice();
        assert_eq!(boxed_slice_item(b), "core::option::Option<&u32>");
    }

    #[test]
    fn expr_fragment_is_narrower() {
        assert_eq!(classify_const_block(), "const-блок");
        assert_eq!(classify_underscore(), "подчёркивание");
    }

    #[test]
    fn mut_resets_binding_mode() {
        assert_eq!(super::bumped(&Some(5)), Some(6));
    }
}
