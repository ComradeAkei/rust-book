//! Раздел 10.4: параметры-константы (const generics).
//!
//! Параметр-константа может стоять в типе только целиком: выражения над
//! ним пока нестабильны (`generic_const_exprs`).
//!
//! ```compile_fail
//! pub fn twice<const N: usize>(a: [u8; N]) -> [u8; N * 2] {
//!     todo!()
//! }
//! ```
//!
//! Тип параметра — только целое, `bool` или `char`:
//!
//! ```compile_fail
//! pub struct Tag<const NAME: &'static str>;
//! ```
//!
//! Выражение в аргументе берётся в фигурные скобки:
//!
//! ```
//! use ch10::const_generic::wrap;
//! assert_eq!(wrap::<{ 1 << 4 }>(17), 1);
//! ```
//!
//! Проверка `const { assert!(..) }` срабатывает при мономорфизации, то
//! есть при сборке (`cargo build`, `cargo test`), но не в `cargo check`:
//!
//! ```compile_fail,E0080
//! use ch10::const_generic::Ring;
//! let r = Ring::<u8, 10>::new();
//! ```

// ANCHOR: wrap
/// Остаток от деления на `N`. `N` — константа, известная в каждой копии
/// функции, и компилятор подбирает для неё лучший код.
pub fn wrap<const N: usize>(i: usize) -> usize {
    i % N
}

#[unsafe(no_mangle)]
pub fn ch10_wrap16(i: usize) -> usize {
    wrap::<16>(i)
}

#[unsafe(no_mangle)]
pub fn ch10_wrap10(i: usize) -> usize {
    wrap::<10>(i)
}

/// То же с делителем, известным только во время работы.
#[unsafe(no_mangle)]
pub fn ch10_wrap_dyn(i: usize, n: usize) -> usize {
    i % n
}
// ANCHOR_END: wrap

// ANCHOR: ring
/// Кольцевой буфер последних `N` отсчётов. Ёмкость — часть типа:
/// `Ring<i16, 64>` и `Ring<i16, 32>` — разные типы, и место под буфер
/// известно при компиляции, без кучи.
#[derive(Debug, Clone)]
pub struct Ring<T, const N: usize = 16> {
    buf: [T; N],
    head: usize,
    len: usize,
}

impl<T: Copy + Default, const N: usize> Ring<T, N> {
    pub const CAPACITY: usize = N;

    pub fn new() -> Self {
        // Вычисляется для каждого N, с которым Ring создают.
        const {
            assert!(N.is_power_of_two(), "ёмкость кольца — степень двойки")
        };
        Ring { buf: [T::default(); N], head: 0, len: 0 }
    }

    /// Добавить отсчёт. Если кольцо полно, самый старый вытесняется и
    /// возвращается.
    pub fn push(&mut self, x: T) -> Option<T> {
        if self.len == N {
            let old = core::mem::replace(&mut self.buf[self.head], x);
            self.head = wrap::<N>(self.head + 1);
            Some(old)
        } else {
            self.buf[wrap::<N>(self.head + self.len)] = x;
            self.len += 1;
            None
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Отсчёты от старого к новому.
    pub fn iter(&self) -> impl Iterator<Item = T> {
        (0..self.len).map(move |i| self.buf[wrap::<N>(self.head + i)])
    }
}

impl<T: Copy + Default, const N: usize> Default for Ring<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
// ANCHOR_END: ring

// ANCHOR: concat
/// Склеить два массива. Длину результата `S` выводит компилятор из
/// аннотации у вызывающего, а `S == A + B` проверяет при мономорфизации:
/// записать `[u8; A + B]` на стабильном Rust нельзя.
pub fn concat<const A: usize, const B: usize, const S: usize>(a: [u8; A], b: [u8; B]) -> [u8; S] {
    const { assert!(S == A + B, "длина результата должна быть A + B") };
    let mut out = [0; S];
    out[..A].copy_from_slice(&a);
    out[A..].copy_from_slice(&b);
    out
}
// ANCHOR_END: concat

// ANCHOR: from-fn
/// Таблица квадратов, построенная при компиляции: длина берётся из типа
/// переменной, в которую таблицу кладут.
pub const fn squares<const N: usize>() -> [u16; N] {
    let mut t = [0u16; N];
    let mut i = 0;
    while i < N {
        t[i] = (i * i) as u16;
        i += 1;
    }
    t
}

pub static SQUARES: [u16; 16] = squares();
// ANCHOR_END: from-fn

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn wrap_is_modulo() {
        for i in 0..100 {
            assert_eq!(ch10_wrap16(i), i % 16);
            assert_eq!(ch10_wrap10(i), i % 10);
            assert_eq!(ch10_wrap_dyn(i, 7), i % 7);
        }
    }

    #[test]
    fn ring_keeps_last_n() {
        let mut r: Ring<i16, 4> = Ring::new();
        assert!(r.is_empty());
        for x in 1..=4 {
            assert_eq!(r.push(x), None);
        }
        assert_eq!(r.push(5), Some(1));
        assert_eq!(r.push(6), Some(2));
        assert_eq!(r.iter().collect::<Vec<_>>(), [3, 4, 5, 6]);
        assert_eq!(r.len(), 4);
        let d: Ring<u8> = Ring::default();
        assert_eq!(Ring::<u8>::CAPACITY, 16);
        assert_eq!(size_of_val(&d), 16 + 2 * size_of::<usize>());
    }

    #[test]
    fn concat_and_tables() {
        let frame: [u8; 5] = concat([0xA5, 0x01], [1, 2, 3]);
        assert_eq!(frame, [0xA5, 0x01, 1, 2, 3]);
        let t: [i32; 4] = core::array::from_fn(|i| (i as i32) * 10);
        assert_eq!(t, [0, 10, 20, 30]);
        assert_eq!([1u8, 2, 3].map(u16::from), [1u16, 2, 3]);
        assert_eq!(SQUARES[15], 225);
        assert_eq!(squares::<3>(), [0, 1, 4]);
    }
}
