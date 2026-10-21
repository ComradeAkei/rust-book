//! Раздел A.11: что в C является неопределённым поведением, а в Rust
//! не компилируется или определено.
//!
//! Ошибки компиляции проверяют doc-тесты `compile_fail` с кодом ошибки,
//! поведение во время выполнения — обычные тесты.

/// Использование после освобождения:
///
/// ```compile_fail,E0382
/// let b = Box::new(5);
/// drop(b); // free(b)
/// println!("{b}");
/// ```
///
/// Висячий указатель на локальную переменную:
///
/// ```compile_fail,E0515
/// fn dangling() -> &'static i32 {
///     let x = 5;
///     &x
/// }
/// ```
///
/// Чтение неинициализированной переменной:
///
/// ```compile_fail,E0381
/// let x: i32;
/// println!("{x}");
/// ```
///
/// Инвалидация итератора (в C++ это `push_back` внутри цикла по вектору,
/// в C — `realloc` буфера, по которому идёт указатель):
///
/// ```compile_fail,E0502
/// let mut v = vec![1, 2, 3];
/// for x in &v {
///     v.push(*x);
/// }
/// ```
pub const COMPILE_ERRORS_ARE_CHECKED_BY_DOCTESTS: () = ();

/* ANCHOR: compile_errors
let b = Box::new(5);
drop(b);
println!("{b}");        // error[E0382]: borrow of moved value: `b`

fn dangling() -> &'static i32 {
    let x = 5;
    &x                  // error[E0515]: cannot return reference to local variable `x`
}

let x: i32;
println!("{x}");        // error[E0381]: used binding `x` isn't initialized

let mut v = vec![1, 2, 3];
for x in &v {
    v.push(*x);         // error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
}
ANCHOR_END: compile_errors */

// ANCHOR: bounds
/// Индекс вне массива: в C запись за границу, здесь проверка и паника.
pub fn nth(v: &[u32], i: usize) -> u32 {
    v[i]
}

/// Без паники: отсутствие значения возвращается явно.
pub fn nth_checked(v: &[u32], i: usize) -> Option<u32> {
    v.get(i).copied()
}
// ANCHOR_END: bounds

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 3 but the index is 10")]
    fn out_of_bounds_panics() {
        nth(&[1, 2, 3], 10);
    }

    #[test]
    fn checked_access() {
        assert_eq!(nth_checked(&[1, 2, 3], 1), Some(2));
        assert_eq!(nth_checked(&[1, 2, 3], 10), None);
    }
}
