//! Задача D: адаптер, который ломает векторизацию, и минимальная правка,
//! которая её возвращает; сколько аллокаций делает `collect`.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(feature = "alloc")]
use core::num::ParseIntError;

// ANCHOR: glue
/// Склейка двух итераторов, своя `chain`. Только `next`: свёртка по
/// умолчанию зовёт его в цикле, и проверка «первый кончился?» остаётся на
/// каждом шаге.
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

/// Правка — один метод: свёртка сворачивает половины по очереди их
/// собственными `fold`.
pub struct GlueFold<A, B>(Glue<A, B>);

pub fn glue_fold<A: Iterator, B: Iterator<Item = A::Item>>(a: A, b: B) -> GlueFold<A, B> {
    GlueFold(glue(a, b))
}

impl<A: Iterator, B: Iterator<Item = A::Item>> Iterator for GlueFold<A, B> {
    type Item = A::Item;

    fn next(&mut self) -> Option<A::Item> {
        self.0.next()
    }

    fn fold<Acc, F: FnMut(Acc, A::Item) -> Acc>(self, init: Acc, mut f: F) -> Acc {
        let Glue { a, b } = self.0;
        let acc = match a {
            Some(a) => a.fold(init, &mut f),
            None => init,
        };
        b.fold(acc, f)
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
    [
        (1, 0), // точная длина из size_hint: одна аллокация
        (1, 7), // у filter нижняя граница 0: 4, 8, …, 512
        (1, 0), // chain из двух точных — точный
        (1, 9), // у flat_map нижняя граница 0: 1500 элементов, 4…2048
        (0, 0), // сборка на месте: тот же буфер, тот же тип
        (1, 0), // u64 шире u32, на месте нельзя
        (1, 8), // collect в Result теряет точную длину: 4…1024
        (1, 8), // как filter, и ещё realloc при сужении до Box<[u32]>
    ]
}
// ANCHOR_END: predicted_allocs

// ANCHOR: once
/// Числа `x`, кратные трём, ровно одной аллокацией и без лишней ёмкости:
/// сначала посчитать, потом выделить.
#[cfg(feature = "alloc")]
pub fn multiples_of_3_once(x: &[u32]) -> Vec<u32> {
    let keep = |v: &&u32| *v % 3 == 0;
    let mut out = Vec::with_capacity(x.iter().filter(keep).count());
    out.extend(x.iter().filter(keep));
    out
}

/// Разбор чисел ровно одной аллокацией: ёмкость известна заранее, а
/// `collect` в `Result` её бы потерял.
#[cfg(feature = "alloc")]
pub fn parse_once(xs: &[&str]) -> Result<Vec<u16>, ParseIntError> {
    let mut out = Vec::with_capacity(xs.len());
    for s in xs {
        out.push(s.parse()?);
    }
    Ok(out)
}
// ANCHOR_END: once
