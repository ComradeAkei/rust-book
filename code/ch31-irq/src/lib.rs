//! Раздел 31.2: логика прерывания отдельно от железа. Крейт без std:
//! его собирает и прошивка `firmware/ch31-tests`, и тесты на хосте.
//!
//! Прерывание АЦП копит отсчёты — сумму и их число. Основной цикл раз в
//! период забирает накопленное и обнуляет. Обе стороны обращаются к
//! общим ячейкам через [`Shared`]: на железе это атомарные load/store и
//! запрет прерываний, в тесте — модель, которая «вызывает прерывание»
//! перед любым выбранным обращением основного цикла.
#![no_std]

/// Ячейки, общие для прерывания и основного цикла.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Sum,
    Count,
}

// ANCHOR: shared
/// Доступ к общим ячейкам — по одному обращению за вызов.
pub trait Shared {
    fn load(&self, cell: Cell) -> u32;
    fn store(&self, cell: Cell, v: u32);
    /// Выполнить `f` с запрещёнными прерываниями.
    fn critical<R>(&self, f: impl FnOnce() -> R) -> R;
}
// ANCHOR_END: shared

// ANCHOR: isr
/// Тело обработчика прерывания АЦП: добавить отсчёт. Основной цикл
/// обработчик не вытесняет, поэтому здесь критическая секция не нужна.
pub fn on_sample(s: &impl Shared, raw: u16) {
    s.store(Cell::Sum, s.load(Cell::Sum).wrapping_add(u32::from(raw)));
    s.store(Cell::Count, s.load(Cell::Count).wrapping_add(1));
}
// ANCHOR_END: isr

// ANCHOR: take
/// Забрать сумму и число отсчётов, первая версия: четыре обращения
/// подряд, между любыми двумя может прийти прерывание.
pub fn take_v1(s: &impl Shared) -> (u32, u32) {
    let sum = s.load(Cell::Sum);
    let count = s.load(Cell::Count);
    s.store(Cell::Sum, 0);
    s.store(Cell::Count, 0);
    (sum, count)
}

/// Те же обращения в критической секции: прерывание ждёт её конца.
pub fn take(s: &impl Shared) -> (u32, u32) {
    s.critical(|| take_v1(s))
}
// ANCHOR_END: take

// ANCHOR: scenario
/// Итог одного сценария: что вернул основной цикл и что осталось в ячейках.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub taken: (u32, u32),
    pub left: (u32, u32),
}

impl Outcome {
    /// Ни один отсчёт не потерян и не раздвоен: взятое плюс оставшееся —
    /// всё, что накопили, по сумме и по числу.
    pub fn conserves(&self, sum: u32, count: u32) -> bool {
        self.taken.0.wrapping_add(self.left.0) == sum
            && self.taken.1.wrapping_add(self.left.1) == count
    }
}
// ANCHOR_END: scenario
