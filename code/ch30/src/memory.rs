//! Раздел 30.7: память — раскладка данных, ложное разделение строк кэша
//! и выделения в горячем цикле.

use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

// ANCHOR: aos
/// Запись журнала станции: 32 байта, из которых среднему значению нужны
/// четыре. Раскладка задана (`repr(C)`): опыт считает байты в строке
/// кэша, и порядок полей не должен зависеть от компилятора.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Reading {
    pub t_ms: u32,
    pub value: i32,
    pub sensor: u16,
    pub flags: u16,
    pub quality: f32,
    pub raw: [u8; 16],
}

const _: () = assert!(size_of::<Reading>() == 32);

/// Журнал строками: массив структур (AoS).
pub fn readings(n: usize) -> Vec<Reading> {
    (0..n)
        .map(|i| Reading {
            t_ms: i as u32 * 10,
            value: (i % 2001) as i32 - 1000,
            sensor: (i % 64) as u16,
            flags: 0,
            quality: 1.0,
            raw: [i as u8; 16],
        })
        .collect()
}

/// Сумма значений по массиву структур: читаются все 32 байта записи,
/// потому что строка кэша — 64 байта, две записи целиком.
#[inline(never)]
pub fn sum_aos(log: &[Reading]) -> i64 {
    log.iter().map(|r| i64::from(r.value)).sum()
}
// ANCHOR_END: aos

// ANCHOR: soa
/// Тот же журнал столбцами: структура массивов (SoA).
pub struct Columns {
    pub t_ms: Vec<u32>,
    pub value: Vec<i32>,
    pub sensor: Vec<u16>,
}

impl Columns {
    pub fn from_rows(log: &[Reading]) -> Columns {
        Columns {
            t_ms: log.iter().map(|r| r.t_ms).collect(),
            value: log.iter().map(|r| r.value).collect(),
            sensor: log.iter().map(|r| r.sensor).collect(),
        }
    }
}

/// Сумма по одному столбцу: в каждой строке кэша шестнадцать значений.
#[inline(never)]
pub fn sum_soa(c: &Columns) -> i64 {
    c.value.iter().map(|&v| i64::from(v)).sum()
}
// ANCHOR_END: soa

// ANCHOR: sharing
/// Два счётчика рядом: в одной строке кэша. Выравнивание на 64 — чтобы
/// пара из 16 байт не легла на границу двух строк (со смещением 56 в
/// строке так вышло бы у каждого восьмого адреса).
#[repr(C, align(64))]
#[derive(Default)]
pub struct Packed {
    pub a: AtomicU64,
    pub b: AtomicU64,
}

/// Счётчик в своей строке. 128, а не 64: соседний предвыборщик Intel
/// тянет строки парами.
#[repr(C, align(128))]
#[derive(Default)]
pub struct Padded(pub AtomicU64);

/// Два счётчика в соседних 64-байтных строках одной 128-байтной пары —
/// проверка, нужен ли запас до 128 на этой машине.
#[repr(C, align(128))]
#[derive(Default)]
pub struct Adjacent {
    pub a: AtomicU64,
    _gap: [u64; 7],
    pub b: AtomicU64,
}

/// Два потока, каждый `n` раз увеличивает свой счётчик.
pub fn bump_pair(a: &AtomicU64, b: &AtomicU64, n: u64) {
    std::thread::scope(|s| {
        s.spawn(|| (0..n).for_each(|_| _ = a.fetch_add(1, Relaxed)));
        s.spawn(|| (0..n).for_each(|_| _ = b.fetch_add(1, Relaxed)));
    });
}
// ANCHOR_END: sharing

// ANCHOR: alloc
/// Строки журнала через `format!`: новая `String` на каждую запись.
#[inline(never)]
pub fn lines_format(log: &[Reading]) -> String {
    let mut out = String::new();
    for r in log {
        let line = format!("{} {} {}\n", r.sensor, r.t_ms, r.value);
        out.push_str(&line);
    }
    out
}

/// Те же строки через `write!` в один буфер: выделения — только когда
/// буфер растёт.
#[inline(never)]
pub fn lines_write(log: &[Reading]) -> String {
    let mut out = String::new();
    for r in log {
        writeln!(out, "{} {} {}", r.sensor, r.t_ms, r.value).unwrap();
    }
    out
}
// ANCHOR_END: alloc

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_results() {
        let log = readings(10_000);
        assert_eq!(sum_aos(&log), sum_soa(&Columns::from_rows(&log)));
        assert_eq!(lines_format(&log[..100]), lines_write(&log[..100]));
        let (a, b) = (Padded::default(), Padded::default());
        bump_pair(&a.0, &b.0, 1000);
        assert_eq!(a.0.load(Relaxed) + b.0.load(Relaxed), 2000);
        assert_eq!(align_of::<Padded>(), 128);
        assert_eq!(align_of::<Packed>(), 64);
        assert_eq!(std::mem::offset_of!(Adjacent, b), 64);
    }
}
