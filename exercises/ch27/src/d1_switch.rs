//! Задача D1: переключатель задач на PendSV. У каждой задачи свой стек и
//! своё значение PSP. Обработчик PendSV сохраняет r4–r11 на стек уходящей
//! задачи, спрашивает у очереди, чья очередь следующая, и восстанавливает
//! её r4–r11; r0–r3, r12, lr, pc и xPSR сохраняет и восстанавливает сам
//! процессор. Обработчик на ассемблере и задачи — в прошивке
//! `firmware/ch27-tasks`, программа `d1_switch`; здесь — то, что
//! проверяется на хосте.

// ANCHOR: frame
/// Слов в кадре задачи: 8 сохраняет PendSV (r4–r11), 8 — процессор (r0–r3,
/// r12, lr, pc, xPSR).
pub const FRAME_WORDS: usize = 16;

/// Бит T регистра xPSR: без него первая же инструкция задачи — UsageFault
/// INVSTATE.
pub const XPSR_T: u32 = 1 << 24;

/// Значение lr в начальном кадре: возврат из задачи — переход по
/// нечётному адресу вне памяти, то есть сбой, а не тихое продолжение.
pub const NO_RETURN: u32 = 0xFFFF_FFFF;

/// Стек задачи. Выравнивание на 8 байт и чётное число слов: вершина и
/// SP после снятия кадра выровнены, как требует AAPCS.
#[repr(C, align(8))]
pub struct Stack<const N: usize> {
    pub words: [u32; N],
}
// ANCHOR_END: frame

impl<const N: usize> Default for Stack<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Stack<N> {
    pub const fn new() -> Self {
        assert!(N >= FRAME_WORDS && N % 2 == 0, "стек — чётное число слов, не меньше кадра");
        Stack { words: [0; N] }
    }

    /// Начальный кадр: стек выглядит так, будто задачу прервал PendSV до
    /// её первой инструкции. r4–r11 — их номера (0x04040404 …), чтобы в
    /// отладчике было видно, что восстановилось; r0 — аргумент задачи; pc
    /// — точка входа без бита Thumb (бит T — в xPSR). Возвращает
    /// начальное значение PSP: адрес сохранённого r4.
    pub fn prepare(&mut self, entry: u32, arg: u32) -> *mut u32 {
        let _ = (entry, arg, NO_RETURN, XPSR_T);
        todo!("задача D1: prepare")
    }
}

/// Очередь задач по кругу. Хранит сохранённые PSP; пока ни одна задача не
/// запущена, текущей нет.
pub struct RoundRobin<const K: usize> {
    sp: [u32; K],
    len: usize,
    current: Option<usize>,
}

impl<const K: usize> Default for RoundRobin<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const K: usize> RoundRobin<K> {
    pub const fn new() -> Self {
        RoundRobin { sp: [0; K], len: 0, current: None }
    }

    /// Добавить задачу с начальным PSP; номер задачи или `Err(sp)`, если
    /// места нет.
    pub fn add(&mut self, sp: u32) -> Result<usize, u32> {
        let _ = (sp, &mut self.sp, &mut self.len);
        todo!("задача D1: add")
    }

    pub fn current(&self) -> Option<usize> {
        todo!("задача D1: current")
    }

    /// Вызывается из PendSV: запомнить PSP уходящей задачи и вернуть PSP
    /// следующей. При первом вызове уходящей задачи нет, и `saved`
    /// отбрасывается. Без задач — паника: переключаться не на что.
    pub fn switch(&mut self, saved: u32) -> u32 {
        let _ = (saved, &mut self.current);
        todo!("задача D1: switch")
    }
}
