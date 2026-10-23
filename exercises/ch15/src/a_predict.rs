//! Задача A: ёмкость `Vec` после последовательности операций и число
//! обращений к распределителю — без запуска, по правилам роста из раздела
//! 15.1.
//!
//! Типы даны, `predict` — `todo!()`. Тесты сверяют предсказание с
//! настоящим `Vec` для элементов размером 1, 4, 2000 и 0 байт. Правила —
//! наблюдение для rustc 1.97: стратегию роста `Vec` не обещает. Если тест
//! не сходится, загляните в `RawVec` в исходниках `alloc`, а не подгоняйте
//! ответ.

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

pub fn predict(elem_size: usize, ops: &[Op]) -> Prediction {
    todo!()
}
