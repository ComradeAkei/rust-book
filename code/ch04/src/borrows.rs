//! Раздел 4.2: перезаимствование, NLL и двухфазные заимствования.
//!
//! Для параметра обобщённого типа неявного перезаимствования нет: `&mut`
//! перемещается, как любое значение без `Copy`.
//!
//! ```compile_fail,E0382
//! fn consume<T>(_: T) {}
//!
//! let mut n = 0u32;
//! let r = &mut n;
//! consume(r);
//! consume(r);
//! ```
//!
//! Двухфазным бывает только `&mut`, которое компилятор вставляет сам: для
//! получателя метода и при неявном перезаимствовании аргумента. Явно
//! написанное `&mut v` — обычное заимствование, и такой вызов отвергается:
//!
//! ```compile_fail,E0502
//! let mut v: Vec<usize> = vec![1];
//! Vec::push(&mut v, v.len());
//! ```
//!
//! И у метода второй фазы не хватает, если аргумент держит ссылку на сам
//! вектор во время вызова:
//!
//! ```compile_fail,E0502
//! let mut v = vec![1u8, 2];
//! v.extend_from_slice(&v);
//! ```

use alloc::vec::Vec;

// ANCHOR: reborrow
fn bump(counter: &mut u32) {
    *counter += 1;
}

pub fn reborrow() -> u32 {
    let mut n = 0;
    let r = &mut n;
    bump(r); // на деле bump(&mut *r): r не перемещается
    bump(r);
    let r2 = &mut *r; // явное перезаимствование
    *r2 += 10; // пока r2 используется, r трогать нельзя
    *r += 100; // r2 больше не нужна, r снова в деле
    n
}
// ANCHOR_END: reborrow

// ANCHOR: nll
/// Заимствование кончается на последнем использовании, а не в конце блока.
pub fn nll(v: &mut Vec<u16>) -> u16 {
    let first = &v[0];
    let copy = *first; // последнее использование first
    v.push(copy); // здесь first уже не живёт
    v[v.len() - 1]
}
// ANCHOR_END: nll

// ANCHOR: two_phase
/// `v.push(v.len())`: сначала `&mut *v` только резервируется, аргумент
/// читает `v`, и лишь затем заимствование становится изменяемым.
pub fn two_phase(v: &mut Vec<usize>) {
    v.push(v.len());
    Vec::push(v, v.len()); // неявное перезаимствование `v` — тоже двухфазное
}
// ANCHOR_END: two_phase

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn borrows() {
        assert_eq!(reborrow(), 112);
        let mut v = vec![7, 8];
        assert_eq!(nll(&mut v), 7);
        let mut w = vec![0, 0];
        two_phase(&mut w);
        assert_eq!(w, [0, 0, 2, 3]);
    }
}
