//! Раздел 13.3: потребители — свёртки, поиск с ранним выходом, `try_fold`
//! и `collect` в разные коллекции.

use core::ops::ControlFlow;

#[cfg(feature = "alloc")]
use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    string::String,
    vec::Vec,
};
#[cfg(feature = "alloc")]
use core::num::ParseIntError;

// ANCHOR: sum
/// `sum` и `product` обобщены по типу результата, и этот тип пишут явно:
/// аннотацией переменной или турбофишем.
pub fn totals(x: &[u16]) -> (u32, u64) {
    let sum: u32 = x.iter().map(|&s| u32::from(s)).sum();
    let product = x.iter().map(|&s| u64::from(s)).product::<u64>();
    (sum, product)
}
// ANCHOR_END: sum

/// Без типа результата `sum` не знает, что строить: `Sum<&i32>`
/// реализуют и `i32`, и `Option<i32>`, и ещё десятки типов.
///
/// ```compile_fail,E0283
/// fn big(data: &[i32]) -> bool {
///     let s = data.iter().sum();
///     s > 10
/// }
/// ```
pub const fn sum_needs_type() {}

// ANCHOR: search
/// Поиск останавливается на первом подходящем элементе и дальше не
/// читает.
pub struct Found<'a> {
    pub first_negative: Option<usize>,
    pub first_big: Option<&'a i16>,
    pub any_saturated: bool,
    pub all_in_range: bool,
    pub last_big: Option<usize>,
}

pub fn search(x: &[i16]) -> Found<'_> {
    Found {
        first_negative: x.iter().position(|&s| s < 0),
        first_big: x.iter().find(|&&s| s > 100),
        any_saturated: x.iter().any(|s| s.unsigned_abs() >= 2047),
        all_in_range: x.iter().all(|&s| (-1000..=1000).contains(&s)),
        last_big: x.iter().rposition(|&s| s > 100),
    }
}
// ANCHOR_END: search

// ANCHOR: minmax
/// Из равных максимумов `max_by_key` отдаёт последний, а `min_by_key` из
/// равных минимумов — первый.
pub fn loudest_quietest(levels: &[(u8, u16)]) -> (Option<u8>, Option<u8>) {
    let loudest = levels.iter().max_by_key(|&&(_, l)| l).map(|&(ch, _)| ch);
    let quietest = levels.iter().min_by_key(|&&(_, l)| l).map(|&(ch, _)| ch);
    (loudest, quietest)
}

/// У `f32` нет `Ord`, и `max()` к нему не применим. Полный порядок даёт
/// `total_cmp`: в нём NaN больше бесконечности. `f32::max` же NaN
/// пропускает.
pub fn max_f32(x: &[f32]) -> (Option<f32>, f32) {
    let total = x.iter().copied().max_by(f32::total_cmp);
    let skip_nan = x.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (total, skip_nan)
}
// ANCHOR_END: minmax

/// `max` требует `Ord`, а у `f32` его нет: NaN не сравним ни с чем.
///
/// ```compile_fail,E0277
/// fn peak(data: &[f32]) -> Option<f32> {
///     data.iter().copied().max()
/// }
/// ```
pub const fn no_max_for_f32() {}

// ANCHOR: try_fold
/// Контрольная сумма байтов в `u8` без переполнения: `try_fold` прерывает
/// обход, как только замыкание вернуло `None`.
pub fn checked_sum(bytes: &[u8]) -> Option<u8> {
    bytes.iter().try_fold(0u8, |acc, &b| acc.checked_add(b))
}

/// Сколько кадров с нагрузками `lens` поместится в буфер ёмкостью `cap`,
/// если каждый занимает `8 + len` байт, и сколько байт они займут.
/// `ControlFlow::Break` выходит из свёртки со значением.
pub fn frames_that_fit(lens: &[u16], cap: usize) -> (usize, usize) {
    let r = lens.iter().enumerate().try_fold(0usize, |used, (i, &len)| {
        let next = used + 8 + usize::from(len);
        if next > cap { ControlFlow::Break((i, used)) } else { ControlFlow::Continue(next) }
    });
    match r {
        ControlFlow::Break(fit) => fit,
        ControlFlow::Continue(used) => (lens.len(), used),
    }
}
// ANCHOR_END: try_fold

// ANCHOR: collect
/// Один поток пар «датчик — значение» в разные коллекции: тип результата
/// выбирает, что строит `collect`.
#[cfg(feature = "alloc")]
pub struct Collected {
    pub values: Vec<i32>,
    pub last_by_sensor: BTreeMap<u16, i32>,
    pub recent: VecDeque<i32>,
    pub tags: String,
    pub ids: Box<[u16]>,
}

#[cfg(feature = "alloc")]
pub fn collect_kinds(readings: &[(u16, i32)]) -> Collected {
    Collected {
        values: readings.iter().map(|&(_, v)| v).collect(),
        // Повторный ключ перезаписывает значение: остаётся последнее.
        last_by_sensor: readings.iter().copied().collect(),
        recent: readings.iter().rev().take(2).map(|&(_, v)| v).collect(),
        tags: readings.iter().map(|&(id, _)| char::from(b'A' + (id % 26) as u8)).collect(),
        ids: readings.iter().map(|&(id, _)| id).collect(),
    }
}
// ANCHOR_END: collect

// ANCHOR: collect_result
/// Список чисел через запятую. Первая же ошибка останавливает `collect`, и
/// вектор не достраивается.
#[cfg(feature = "alloc")]
pub fn parse_list(s: &str) -> Result<Vec<u16>, ParseIntError> {
    s.split(',').map(|t| t.trim().parse::<u16>()).collect()
}

/// Сумма тех же чисел без вектора: `Sum` реализован и для `Result`.
#[cfg(feature = "alloc")]
pub fn sum_list(s: &str) -> Result<u32, ParseIntError> {
    s.split(',').map(|t| t.trim().parse::<u32>()).sum()
}

/// Ответили ли все датчики: `None` хоть у одного — `None` у всех.
#[cfg(feature = "alloc")]
pub fn all_present(xs: &[Option<i16>]) -> Option<Vec<i16>> {
    xs.iter().copied().collect()
}
// ANCHOR_END: collect_result

// ANCHOR: partition
/// `partition` раскладывает элементы в две коллекции по условию, `unzip`
/// — пары в две коллекции по компонентам, `extend` дописывает в
/// существующую.
#[cfg(feature = "alloc")]
pub struct Split {
    pub non_negative: Vec<i32>,
    pub negative: Vec<i32>,
    pub ids: Vec<u16>,
    pub nonzero: Vec<i32>,
}

#[cfg(feature = "alloc")]
pub fn split_up(readings: &[(u16, i32)]) -> Split {
    let (non_negative, negative) = readings.iter().map(|&(_, v)| v).partition(|&v| v >= 0);
    let (more_ids, values): (Vec<u16>, Vec<i32>) = readings.iter().copied().unzip();
    let mut ids = Vec::from([0u16]);
    ids.extend(more_ids);
    let mut nonzero = Vec::new();
    nonzero.extend(values.iter().filter(|&&v| v != 0)); // Extend<&i32> у Vec<i32>
    Split { non_negative, negative, ids, nonzero }
}
// ANCHOR_END: partition

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    #[test]
    fn sums() {
        assert_eq!(totals(&[1000, 2000, 3]), (3003, 6_000_000));
        assert_eq!(totals(&[]), (0, 1));
    }

    #[test]
    fn searches() {
        let x = [5, 0, 300, -7, 150, 2];
        let f = search(&x);
        assert_eq!(f.first_negative, Some(3));
        assert_eq!(f.first_big, Some(&300));
        assert!(!f.any_saturated && f.all_in_range);
        assert_eq!(f.last_big, Some(4));
        let f = search(&[2000, -2047]);
        assert!(f.any_saturated && !f.all_in_range);
        // Ранний выход: any читает элементы только до первого
        // отрицательного.
        let reads = Cell::new(0);
        let hit = x.iter().inspect(|_| reads.set(reads.get() + 1)).any(|&s| s < 0);
        assert!(hit);
        assert_eq!(reads.get(), 4);
    }

    #[test]
    fn min_max() {
        let levels = [(1, 70), (2, 90), (3, 10), (4, 90), (5, 10)];
        assert_eq!(loudest_quietest(&levels), (Some(4), Some(3)));
        assert_eq!(loudest_quietest(&[]), (None, None));
        let (total, skip) = max_f32(&[1.0, f32::NAN, 3.0]);
        assert!(total.unwrap().is_nan());
        assert_eq!(skip, 3.0);
        assert_eq!(max_f32(&[-0.0, 0.0]).0.map(f32::is_sign_positive), Some(true));
        assert_eq!(max_f32(&[]), (None, f32::NEG_INFINITY));
    }

    #[test]
    fn try_folds() {
        assert_eq!(checked_sum(&[100, 100, 55]), Some(255));
        assert_eq!(checked_sum(&[100, 100, 56]), None);
        assert_eq!(frames_that_fit(&[10, 0, 240, 4], 200), (2, 26));
        assert_eq!(frames_that_fit(&[10, 0, 240, 4], 300), (4, 286));
        assert_eq!(frames_that_fit(&[10, 0], 300), (2, 26));
        assert_eq!(frames_that_fit(&[], 0), (0, 0));
        // Обход прервался на первом переполнении и дальше не читал.
        let reads = Cell::new(0);
        let r = [200u8, 100, 1, 1]
            .iter()
            .inspect(|_| reads.set(reads.get() + 1))
            .try_fold(0u8, |a, &b| a.checked_add(b));
        assert_eq!((r, reads.get()), (None, 2));
    }

    #[test]
    fn collects() {
        let r = [(3, 10), (1, -5), (3, 12), (27, 0)];
        let c = collect_kinds(&r);
        assert_eq!(c.values, [10, -5, 12, 0]);
        assert_eq!(c.last_by_sensor.into_iter().collect::<Vec<_>>(), [(1, -5), (3, 12), (27, 0)]);
        assert_eq!(c.recent, [0, 12]);
        assert_eq!(c.tags, "DBDB");
        assert_eq!(&*c.ids, [3, 1, 3, 27]);
    }

    #[test]
    fn collect_results() {
        assert_eq!(parse_list("1, 2,30"), Ok(vec![1, 2, 30]));
        assert_eq!(
            parse_list("1,x,70000").unwrap_err().to_string(),
            "invalid digit found in string"
        );
        assert_eq!(
            parse_list("1,70000,x").unwrap_err().to_string(),
            "number too large to fit in target type"
        );
        assert_eq!(sum_list("1, 2,30"), Ok(33));
        assert!(sum_list("1,,2").is_err());
        assert_eq!(all_present(&[Some(1), Some(2)]), Some(vec![1, 2]));
        assert_eq!(all_present(&[Some(1), None]), None);
        assert_eq!(all_present(&[]), Some(vec![]));
        // collect в Result останавливает разбор на первой ошибке.
        let parsed = Cell::new(0);
        let r: Result<Vec<u8>, _> = ["1", "x", "3"]
            .iter()
            .inspect(|_| parsed.set(parsed.get() + 1))
            .map(|t| t.parse::<u8>())
            .collect();
        assert!(r.is_err());
        assert_eq!(parsed.get(), 2);
    }

    #[test]
    fn partitions() {
        let r = [(3, 10), (1, -5), (3, 0), (7, -1)];
        let s = split_up(&r);
        assert_eq!((s.non_negative, s.negative), (vec![10, 0], vec![-5, -1]));
        assert_eq!(s.ids, [0, 3, 1, 3, 7]);
        assert_eq!(s.nonzero, [10, -5, -1]);
    }
}
