//! Задача A: ёмкость `Vec` после последовательности операций и число
//! обращений к распределителю — без запуска, по правилам роста из раздела
//! 15.1.
//!
//! Правила — наблюдение для rustc 1.97 (`RawVec::grow_amortized`), не
//! гарантия: стратегию роста `Vec` не обещает. Тесты сверяют предсказание с
//! настоящим `Vec`.

// ANCHOR: types
/// Операция над `Vec<T>` с элементом размера `elem_size`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `n` раз `push`.
    Push(usize),
    /// `extend_from_slice` из `n` элементов: длина известна заранее.
    ExtendExact(usize),
    /// `extend` из `n` элементов итератора без размера, как у `filter`.
    ExtendUnknown(usize),
    Reserve(usize),
    ReserveExact(usize),
    ShrinkToFit,
    ShrinkTo(usize),
    Truncate(usize),
    Clear,
}

/// Длина и ёмкость в конце и сколько раз `Vec` позвал `alloc`, `realloc`
/// и `dealloc`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Prediction {
    pub len: usize,
    pub cap: usize,
    pub allocs: usize,
    pub reallocs: usize,
    pub deallocs: usize,
}
// ANCHOR_END: types

// ANCHOR: solution
/// Наименьшая ненулевая ёмкость: короткие векторы растут не с единицы.
fn min_cap(elem_size: usize) -> usize {
    match elem_size {
        1 => 8,
        2..=1024 => 4,
        _ => 1,
    }
}

impl Prediction {
    /// Буфер становится ёмкостью `cap`: из пустого — `alloc`, в пустой —
    /// `dealloc`, иначе — `realloc`.
    fn resize(&mut self, cap: usize) {
        match (self.cap, cap) {
            (0, 0) => {}
            (0, _) => self.allocs += 1,
            (_, 0) => self.deallocs += 1,
            (a, b) if a != b => self.reallocs += 1,
            _ => {}
        }
        self.cap = cap;
    }

    /// Места на `additional` элементов сверх длины: если не хватает —
    /// удвоение, но не меньше нужного и не меньше `min_cap`.
    fn reserve(&mut self, additional: usize, elem_size: usize) {
        if self.cap - self.len < additional {
            let need = self.len + additional;
            self.resize((self.cap * 2).max(need).max(min_cap(elem_size)));
        }
    }

    fn reserve_exact(&mut self, additional: usize) {
        if self.cap - self.len < additional {
            self.resize(self.len + additional);
        }
    }

    /// `shrink_to(min)`: сжатие до `max(len, min)`, если ёмкость больше
    /// `min`, — даже когда `len` и так равна ёмкости: тогда `realloc`
    /// зовётся с прежним размером. Пустой буфер освобождается.
    fn shrink_to(&mut self, min: usize) {
        if self.cap > min {
            let target = self.len.max(min);
            if target == self.cap {
                self.reallocs += 1;
            }
            self.resize(target);
        }
    }
}

pub fn predict(elem_size: usize, ops: &[Op]) -> Prediction {
    let mut p = Prediction::default();
    if elem_size == 0 {
        // Нулевой размер: буфер не нужен, ёмкость бесконечна.
        p.cap = usize::MAX;
        for op in ops {
            match *op {
                Op::Push(n) | Op::ExtendExact(n) | Op::ExtendUnknown(n) => p.len += n,
                Op::Truncate(n) => p.len = p.len.min(n),
                Op::Clear => p.len = 0,
                _ => {}
            }
        }
        return p;
    }
    for op in ops {
        match *op {
            // Итератор без размера растит вектор так же, как push по одному.
            Op::Push(n) | Op::ExtendUnknown(n) => {
                for _ in 0..n {
                    p.reserve(1, elem_size);
                    p.len += 1;
                }
            }
            Op::ExtendExact(n) => {
                p.reserve(n, elem_size);
                p.len += n;
            }
            Op::Reserve(n) => p.reserve(n, elem_size),
            Op::ReserveExact(n) => p.reserve_exact(n),
            Op::ShrinkToFit => {
                if p.cap > p.len {
                    p.resize(p.len);
                }
            }
            Op::ShrinkTo(n) => p.shrink_to(n),
            Op::Truncate(n) => p.len = p.len.min(n),
            Op::Clear => p.len = 0,
        }
    }
    p
}
// ANCHOR_END: solution
