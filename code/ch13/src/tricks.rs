//! Раздел 13.6: трюки и ловушки — порядок `rev` и `enumerate`, байт,
//! который съедает `take_while`, молчаливый `zip`, переполнение `sum`,
//! побочные эффекты в ленивой цепочке и нулевой шаг.

use core::cell::Cell;

// ANCHOR: rev_enumerate
/// `rev().enumerate()` нумерует заново, с нуля от конца. `enumerate().rev()`
/// идёт с конца, но номера сохраняет исходные.
pub type Numbered = [(usize, char); 3];

pub fn orders() -> (Numbered, Numbered) {
    let s = ['a', 'b', 'c'];
    let mut renumbered = [(0, ' '); 3];
    let mut original = [(0, ' '); 3];
    for (slot, p) in renumbered.iter_mut().zip(s.iter().copied().rev().enumerate()) {
        *slot = p; // (0, 'c'), (1, 'b'), (2, 'a')
    }
    for (slot, p) in original.iter_mut().zip(s.iter().copied().enumerate().rev()) {
        *slot = p; // (2, 'c'), (1, 'b'), (0, 'a')
    }
    (renumbered, original)
}
// ANCHOR_END: rev_enumerate

/// Чтобы идти с конца с исходными номерами, `enumerate` должен знать
/// длину заранее: нужен `ExactSizeIterator`. У `Chars` его нет — длину в
/// символах не узнать, не пройдя строку.
///
/// ```compile_fail,E0277
/// fn last_char(s: &str) -> Option<(usize, char)> {
///     s.chars().enumerate().rev().next()
/// }
/// ```
pub const fn enumerate_rev_needs_len() {}

// ANCHOR: take_while
/// Поля через запятую, «12,34». `take_while` забирает из итератора и
/// первый байт, на котором условие ложно, — запятую, и она пропадает.
pub fn first_field_take_while(s: &[u8]) -> (u32, Option<u8>) {
    let mut it = s.iter();
    let first =
        it.by_ref().take_while(|&&b| b != b',').fold(0, |n, &b| n * 10 + u32::from(b - b'0'));
    (first, it.next().copied()) // (12, Some(b'3')): запятой уже нет
}

/// `peekable` и `next_if` забирают байт, только если он подходит: запятая
/// остаётся в итераторе.
pub fn first_field_next_if(s: &[u8]) -> (u32, Option<u8>) {
    let mut it = s.iter().peekable();
    let mut first = 0;
    while let Some(&b) = it.next_if(|&&b| b != b',') {
        first = first * 10 + u32::from(b - b'0');
    }
    (first, it.next().copied()) // (12, Some(b','))
}
// ANCHOR_END: take_while

// ANCHOR: zip_len
/// `zip` молча останавливается на конце более короткого: несовпадение
/// длин не видно. `Iterator::eq` сравнивает и длины.
pub fn same_bytes(a: &[u8], b: &[u8]) -> (bool, bool) {
    let zipped = a.iter().zip(b).all(|(x, y)| x == y);
    let equal = a.iter().eq(b);
    (zipped, equal)
}
// ANCHOR_END: zip_len

// ANCHOR: overflow
/// `sum` складывает в типе результата, и сумма байтов в `u8`
/// переполняется: в отладочной сборке это паника, в release — перенос.
pub fn byte_sum(x: &[u8]) -> u8 {
    x.iter().sum()
}

/// Расширять надо до сложения, а не после.
pub fn byte_sum_wide(x: &[u8]) -> u32 {
    x.iter().map(|&b| u32::from(b)).sum()
}
// ANCHOR_END: overflow

// ANCHOR: side_effects
/// Побочный эффект в `map` случается только тогда, когда потребитель
/// просит элемент. `let _ =` глушит предупреждение, но не запускает
/// цепочку; `peek` запускает её на шаг раньше `next`.
pub fn effects() -> [usize; 4] {
    let calls = Cell::new(0);
    let tick = |x: &u8| {
        calls.set(calls.get() + 1);
        *x
    };
    let data = [1u8, 2, 3];
    let _ = data.iter().map(tick);
    let silent = calls.get(); //               0
    data.iter().map(tick).for_each(drop);
    let all = calls.get(); //                  3
    let mut p = data.iter().map(tick).peekable();
    p.peek();
    let peeked = calls.get(); //               4: замыкание уже вызвано
    p.next();
    p.next();
    [silent, all, peeked, calls.get()] //      5: второй next — второй вызов
}
// ANCHOR_END: side_effects

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rev_and_enumerate() {
        let (a, b) = orders();
        assert_eq!(a, [(0, 'c'), (1, 'b'), (2, 'a')]);
        assert_eq!(b, [(2, 'c'), (1, 'b'), (0, 'a')]);
    }

    #[test]
    fn take_while_eats_separator() {
        assert_eq!(first_field_take_while(b"12,34"), (12, Some(b'3')));
        assert_eq!(first_field_next_if(b"12,34"), (12, Some(b',')));
        assert_eq!(first_field_take_while(b"12"), (12, None));
        assert_eq!(first_field_next_if(b"12"), (12, None));
    }

    #[test]
    fn zip_is_silent() {
        assert_eq!(same_bytes(b"\xa5\x01", b"\xa5\x01\x03"), (true, false));
        assert_eq!(same_bytes(b"\xa5\x01", b"\xa5\x01"), (true, true));
        assert_eq!(same_bytes(b"\xa5\x02", b"\xa5\x01"), (false, false));
    }

    #[test]
    fn sum_overflows_in_result_type() {
        let r = std::panic::catch_unwind(|| byte_sum(&[200, 100]));
        if cfg!(debug_assertions) {
            assert!(r.is_err());
        } else {
            assert_eq!(r.unwrap(), 44);
        }
        assert_eq!(byte_sum_wide(&[200, 100]), 300);
    }

    #[test]
    fn lazy_effects() {
        assert_eq!(effects(), [0, 3, 4, 5]);
    }

    #[test]
    #[should_panic(expected = "window size must be non-zero")]
    fn zero_window() {
        let _ = [1, 2, 3].windows(std::hint::black_box(0)).count();
    }

    #[test]
    #[should_panic(expected = "chunk size must be non-zero")]
    fn zero_chunk() {
        let _ = [1, 2, 3].chunks(std::hint::black_box(0)).count();
    }

    #[test]
    #[should_panic(expected = "assertion failed: step != 0")]
    fn zero_step() {
        let _ = (0..3).step_by(std::hint::black_box(0)).count();
    }
}
