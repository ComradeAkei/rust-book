//! Раздел 31.2: детерминированный тест логики прерывания на хосте.
//! Модель ячеек «вызывает прерывание» перед обращением номер `fire_at`
//! основного цикла; тест перебирает все места.

use ch31_irq::{Cell, Outcome, Shared, on_sample, take, take_v1};
use std::cell::Cell as StdCell;

// ANCHOR: model
/// Две ячейки и одно прерывание АЦП с отсчётом 7, которое придёт перед
/// обращением номер `fire_at` основного цикла. Под запретом прерываний
/// оно ждёт, как ждёт на железе: бит в регистре ожидания, а не вызов.
struct Model {
    sum: StdCell<u32>,
    count: StdCell<u32>,
    accesses: StdCell<usize>,
    fire_at: usize,
    pending: StdCell<bool>,
    masked: StdCell<bool>,
    in_isr: StdCell<bool>,
}

impl Model {
    fn new(sum: u32, count: u32, fire_at: usize) -> Self {
        Model {
            sum: StdCell::new(sum),
            count: StdCell::new(count),
            accesses: StdCell::new(0),
            fire_at,
            pending: StdCell::new(false),
            masked: StdCell::new(false),
            in_isr: StdCell::new(false),
        }
    }

    /// Перед каждым обращением основного цикла.
    fn tick(&self) {
        if self.in_isr.get() {
            return;
        }
        let n = self.accesses.replace(self.accesses.get() + 1);
        if n == self.fire_at {
            self.pending.set(true);
        }
        if self.pending.get() && !self.masked.get() {
            self.isr();
        }
    }

    fn isr(&self) {
        self.pending.set(false);
        self.in_isr.set(true);
        on_sample(self, 7);
        self.in_isr.set(false);
    }

    fn cell(&self, c: Cell) -> &StdCell<u32> {
        match c {
            Cell::Sum => &self.sum,
            Cell::Count => &self.count,
        }
    }
}

impl Shared for Model {
    fn load(&self, c: Cell) -> u32 {
        self.tick();
        self.cell(c).get()
    }
    fn store(&self, c: Cell, v: u32) {
        self.tick();
        self.cell(c).set(v);
    }
    fn critical<R>(&self, f: impl FnOnce() -> R) -> R {
        let was = self.masked.replace(true);
        let r = f();
        self.masked.set(was);
        if !was && self.pending.get() {
            self.isr();
        }
        r
    }
}
// ANCHOR_END: model

// ANCHOR: run
/// Накоплено 100 за 10 отсчётов; прерывание добавит 7 перед обращением
/// `fire_at`, а если до него не дошло — сразу после `take`.
fn run(take: fn(&Model) -> (u32, u32), fire_at: usize) -> Outcome {
    let m = Model::new(100, 10, fire_at);
    let taken = take(&m);
    if m.accesses.get() <= fire_at || m.pending.get() {
        m.isr();
    }
    Outcome { taken, left: (m.sum.get(), m.count.get()) }
}

#[test]
fn take_v1_loses_samples() {
    let lost: Vec<usize> = (0..=4).filter(|&k| !run(take_v1, k).conserves(107, 11)).collect();
    assert_eq!(lost, [1, 2, 3]);
    assert_eq!(run(take_v1, 1), Outcome { taken: (100, 11), left: (0, 0) });
    assert_eq!(run(take_v1, 2), Outcome { taken: (100, 10), left: (0, 0) });
    assert_eq!(run(take_v1, 3), Outcome { taken: (100, 10), left: (7, 0) });
}

#[test]
fn take_in_critical_section_conserves() {
    for k in 0..=4 {
        let o = run(take, k);
        assert!(o.conserves(107, 11), "прерывание перед обращением {k}: {o:?}");
    }
    assert_eq!(run(take, 1), Outcome { taken: (100, 10), left: (7, 1) });
}
// ANCHOR_END: run
