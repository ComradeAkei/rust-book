//! Раздел 30.6: цена проверки переполнения и флага `RefCell`.

use std::cell::{Cell, RefCell};

// ANCHOR: overflow
/// Сумма по модулю 2³²: так `+` работает в release.
#[inline(never)]
pub fn sum_wrapping(v: &[u32]) -> u32 {
    v.iter().fold(0, |s, &x| s.wrapping_add(x))
}

/// Сумма с проверкой каждого сложения: так `+` работает в debug и с
/// `overflow-checks = true` — паника на первом переполнении.
#[inline(never)]
pub fn sum_strict(v: &[u32]) -> u32 {
    v.iter().fold(0, |s, &x| s.strict_add(x))
}

/// Тот же ответ одной проверкой в конце. Слагаемые неотрицательны, и
/// частичные суммы только растут: переполнилась одна из них — значит,
/// и полная сумма больше `u32::MAX`. В `u64` полная сумма точна, пока
/// элементов меньше 2³².
#[inline(never)]
pub fn sum_wide(v: &[u32]) -> Option<u32> {
    u32::try_from(v.iter().map(|&x| u64::from(x)).sum::<u64>()).ok()
}
// ANCHOR_END: overflow

// ANCHOR: refcell
/// Гистограмма в `RefCell`: заимствование на каждом байте.
#[inline(never)]
pub fn hist_refcell(data: &[u8], h: &RefCell<[u32; 256]>) {
    for &b in data {
        h.borrow_mut()[usize::from(b)] += 1;
    }
}

/// Одно заимствование на весь цикл.
#[inline(never)]
pub fn hist_refcell_once(data: &[u8], h: &RefCell<[u32; 256]>) {
    let mut h = h.borrow_mut();
    for &b in data {
        h[usize::from(b)] += 1;
    }
}

/// `Cell`: флага нет вовсе, `get` и `set` — обычные чтение и запись.
#[inline(never)]
pub fn hist_cell(data: &[u8], h: &[Cell<u32>; 256]) {
    for &b in data {
        let c = &h[usize::from(b)];
        c.set(c.get() + 1);
    }
}
// ANCHOR_END: refcell

// ANCHOR: refcell_call
/// Заимствование держится на время вызова чужой функции: она могла бы
/// заглянуть в тот же `RefCell`, и флаг обязан быть в памяти.
#[inline(never)]
pub fn hist_refcell_call(data: &[u8], h: &RefCell<[u32; 256]>, weight: fn(u8) -> u32) {
    for &b in data {
        let mut h = h.borrow_mut();
        let w = weight(b);
        h[usize::from(b)] += w;
    }
}

/// То же с `Cell`: вызов между выбором счётчика и записью, флага нет.
#[inline(never)]
pub fn hist_cell_call(data: &[u8], h: &[Cell<u32>; 256], weight: fn(u8) -> u32) {
    for &b in data {
        let c = &h[usize::from(b)];
        let w = weight(b);
        c.set(c.get() + w);
    }
}
// ANCHOR_END: refcell_call

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_sums() {
        // Сумма 0..90 000 — около 4,05·10⁹, ещё меньше u32::MAX.
        let v: Vec<u32> = (0..90_000).collect();
        let want = sum_wrapping(&v);
        assert_eq!(sum_strict(&v), want);
        assert_eq!(sum_wide(&v), Some(want));
    }

    #[test]
    fn wide_sees_the_overflow() {
        let v = [u32::MAX, 1, 0];
        assert_eq!(sum_wide(&v), None);
        assert_eq!(sum_wrapping(&v), 0);
    }

    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn strict_panics() {
        sum_strict(&[u32::MAX, 1]);
    }

    #[test]
    fn same_histograms() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 7919 % 251) as u8).collect();
        let (a, b) = (RefCell::new([0u32; 256]), RefCell::new([0u32; 256]));
        hist_refcell(&data, &a);
        hist_refcell_once(&data, &b);
        let c: [Cell<u32>; 256] = std::array::from_fn(|_| Cell::new(0));
        hist_cell(&data, &c);
        assert_eq!(*a.borrow(), *b.borrow());
        assert_eq!(c.map(Cell::into_inner), *a.borrow());
    }

    #[test]
    fn same_weighted_histograms() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 7919 % 251) as u8).collect();
        let weight: fn(u8) -> u32 = |b| u32::from(b & 1) + 1;
        let a = RefCell::new([0u32; 256]);
        hist_refcell_call(&data, &a, weight);
        let c: [Cell<u32>; 256] = std::array::from_fn(|_| Cell::new(0));
        hist_cell_call(&data, &c, weight);
        assert_eq!(c.map(Cell::into_inner), *a.borrow());
    }

    #[test]
    #[should_panic(expected = "RefCell already borrowed")]
    fn borrowed_twice_panics() {
        let h = RefCell::new([0u32; 256]);
        let _guard = h.borrow();
        hist_refcell(&[1, 2, 3], &h);
    }
}
