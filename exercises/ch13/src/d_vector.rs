//! Задача D: адаптер, который ломает векторизацию, и минимальная правка,
//! которая её возвращает; сколько аллокаций делает `collect`.
//!
//! `Glue`, `ch13d_broken` и `ch13d_fixed` даны. `GlueFold` пока ведёт себя
//! как `Glue`, и тест не находит векторной суммы в `ch13d_fixed`: добавьте
//! в `impl Iterator for GlueFold` один метод так, чтобы она появилась, и
//! объясните в комментарии, почему без него её нет. Затем предскажите
//! аллокации восьми цепочек `CHAINS` и напишите `multiples_of_3_once` и
//! `parse_once`, которые выделяют память ровно один раз.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "alloc")]
use core::num::ParseIntError;

// ANCHOR: glue
/// Склейка двух итераторов, своя `chain`. Реализован только `next`.
pub struct Glue<A, B> {
    a: Option<A>,
    b: B,
}

pub fn glue<A: Iterator, B: Iterator<Item = A::Item>>(a: A, b: B) -> Glue<A, B> {
    Glue { a: Some(a), b }
}

impl<A: Iterator, B: Iterator<Item = A::Item>> Iterator for Glue<A, B> {
    type Item = A::Item;

    fn next(&mut self) -> Option<A::Item> {
        if let Some(a) = &mut self.a {
            if let Some(x) = a.next() {
                return Some(x);
            }
            self.a = None;
        }
        self.b.next()
    }
}

/// Та же склейка: сюда и нужна правка.
pub struct GlueFold<A, B>(Glue<A, B>);

pub fn glue_fold<A: Iterator, B: Iterator<Item = A::Item>>(a: A, b: B) -> GlueFold<A, B> {
    GlueFold(glue(a, b))
}

impl<A: Iterator, B: Iterator<Item = A::Item>> Iterator for GlueFold<A, B> {
    type Item = A::Item;

    fn next(&mut self) -> Option<A::Item> {
        self.0.next()
    }
}
// ANCHOR_END: glue

// ANCHOR: broken
/// Сумма `x` по модулю 2^32 через `Glue`: половины склеены обратно.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13d_broken(x: &[u32]) -> u32 {
    let (l, r) = x.split_at(x.len() / 2);
    glue(l.iter(), r.iter()).fold(0, |s, v| s.wrapping_add(*v))
}

/// То же через `GlueFold`.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13d_fixed(x: &[u32]) -> u32 {
    let (l, r) = x.split_at(x.len() / 2);
    glue_fold(l.iter(), r.iter()).fold(0, |s, v| s.wrapping_add(*v))
}
// ANCHOR_END: broken

// ANCHOR: chains
/// Цепочка и её текст для сообщения теста.
#[cfg(feature = "alloc")]
pub type Chain = (&'static str, fn(Vec<u32>));

/// Восемь `collect` по вектору `v` из 1000 чисел `0..1000`. Каждая
/// функция получает вектор готовым и уничтожает результат сама: тест
/// считает обращения к распределителю только внутри неё. Первая цепочка
/// нарочно не `v.to_vec()`: её надо сравнить с остальными.
#[cfg(feature = "alloc")]
#[allow(clippy::iter_cloned_collect)]
pub const CHAINS: [Chain; 8] = [
    ("v.iter().copied().collect::<Vec<u32>>()", |v| drop(v.iter().copied().collect::<Vec<u32>>())),
    ("v.iter().filter(|&&x| x % 3 == 0).collect::<Vec<_>>()", |v| {
        drop(v.iter().filter(|&&x| x % 3 == 0).collect::<Vec<_>>())
    }),
    ("v.iter().chain(&v).copied().collect::<Vec<u32>>()", |v| {
        drop(v.iter().chain(&v).copied().collect::<Vec<u32>>())
    }),
    ("v.iter().flat_map(|&x| 0..x % 4).collect::<Vec<u32>>()", |v| {
        drop(v.iter().flat_map(|&x| 0..x % 4).collect::<Vec<u32>>())
    }),
    ("v.into_iter().map(|x| x + 1).collect::<Vec<u32>>()", |v| {
        drop(v.into_iter().map(|x| x + 1).collect::<Vec<u32>>())
    }),
    ("v.into_iter().map(u64::from).collect::<Vec<u64>>()", |v| {
        drop(v.into_iter().map(u64::from).collect::<Vec<u64>>())
    }),
    ("v.iter().map(|&x| u16::try_from(x)).collect::<Result<Vec<u16>, _>>()", |v| {
        drop(v.iter().map(|&x| u16::try_from(x)).collect::<Result<Vec<u16>, _>>())
    }),
    ("v.iter().filter(|&&x| x % 3 == 0).copied().collect::<Box<[u32]>>()", |v| {
        drop(v.iter().filter(|&&x| x % 3 == 0).copied().collect::<Box<[u32]>>())
    }),
];
// ANCHOR_END: chains

// ANCHOR: predicted_allocs
/// Сколько раз каждая цепочка из `CHAINS` вызывает `alloc` и `realloc`.
#[cfg(feature = "alloc")]
pub const fn predicted_allocs() -> [(usize, usize); 8] {
    panic!("предскажите без компилятора, затем сверьте с тестом task_d_allocs")
}
// ANCHOR_END: predicted_allocs

// ANCHOR: once
/// Числа `x`, кратные трём, ровно одной аллокацией и без лишней ёмкости.
#[cfg(feature = "alloc")]
pub fn multiples_of_3_once(x: &[u32]) -> Vec<u32> {
    todo!()
}

/// Разбор чисел ровно одной аллокацией; ошибка — первая встреченная.
#[cfg(feature = "alloc")]
pub fn parse_once(xs: &[&str]) -> Result<Vec<u16>, ParseIntError> {
    todo!()
}
// ANCHOR_END: once
