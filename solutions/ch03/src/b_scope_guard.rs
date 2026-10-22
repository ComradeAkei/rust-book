//! Задача 3.B. Страж области видимости и транзакция над регистрами.

use ch03::regs::{NREGS, Regs};
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

// ANCHOR: guard
/// Значение и действие, которое выполнится над ним при выходе из области
/// видимости: обычном, через `return`, `?`, `break` или при панике.
///
/// Оба поля в `ManuallyDrop`: деструктор стража забирает их по значению,
/// а `into_inner` — без деструктора. Флага «обезврежен» нет, поэтому страж
/// не больше, чем значение и замыкание вместе.
pub struct ScopeGuard<T, F: FnOnce(T)> {
    value: ManuallyDrop<T>,
    on_exit: ManuallyDrop<F>,
}

impl<T, F: FnOnce(T)> ScopeGuard<T, F> {
    #[must_use = "страж без имени уничтожается сразу"]
    pub fn new(value: T, on_exit: F) -> Self {
        ScopeGuard { value: ManuallyDrop::new(value), on_exit: ManuallyDrop::new(on_exit) }
    }

    /// Обезвредить стража: вернуть значение, не выполняя действие.
    pub fn into_inner(guard: Self) -> T {
        // Деструктор стража выполниться не должен: он вызвал бы действие.
        let mut guard = ManuallyDrop::new(guard);
        // SAFETY: guard больше не используется и не уничтожается, поэтому
        // каждое поле забирается или уничтожается ровно один раз. Значение
        // забираем первым: если деструктор замыкания запаникует, значение
        // уже лежит в локальной переменной и будет уничтожено раскруткой.
        unsafe {
            let value = ManuallyDrop::take(&mut guard.value);
            ManuallyDrop::drop(&mut guard.on_exit);
            value
        }
    }
}

impl<T, F: FnOnce(T)> Deref for ScopeGuard<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T, F: FnOnce(T)> DerefMut for ScopeGuard<T, F> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T, F: FnOnce(T)> Drop for ScopeGuard<T, F> {
    fn drop(&mut self) {
        // SAFETY: `drop` вызывается один раз, и после него к полям никто не
        // обращается. `into_inner` сюда не попадает: там страж обёрнут в
        // ManuallyDrop.
        let (value, on_exit) =
            unsafe { (ManuallyDrop::take(&mut self.value), ManuallyDrop::take(&mut self.on_exit)) };
        on_exit(value);
    }
}
// ANCHOR_END: guard

// ANCHOR: transaction
/// Старые значения регистров, которые транзакция успела изменить, в
/// порядке первого изменения. Каждый регистр записан не больше одного
/// раза, поэтому шестнадцати ячеек хватает всегда.
struct Journal {
    old: [(u8, u32); NREGS],
    len: usize,
    touched: u16,
}

type State<'a> = (&'a mut Regs, Journal);

/// Откат: записать старые значения в обратном порядке. Последним
/// восстанавливается регистр, который транзакция тронула первым: обычно
/// это «включить устройство», и его логично вернуть после настроек.
fn rollback((regs, journal): State<'_>) {
    for &(i, v) in journal.old[..journal.len].iter().rev() {
        regs.write(i as usize, v);
    }
}

/// Транзакция над регистрами: пишет сразу, но помнит старые значения.
/// Если её не подтвердили, при уничтожении все изменения откатываются.
pub struct Transaction<'a> {
    guard: ScopeGuard<State<'a>, fn(State<'a>)>,
}

impl<'a> Transaction<'a> {
    pub fn begin(regs: &'a mut Regs) -> Self {
        let journal = Journal { old: [(0, 0); NREGS], len: 0, touched: 0 };
        Transaction { guard: ScopeGuard::new((regs, journal), rollback) }
    }

    pub fn read(&self, i: usize) -> u32 {
        self.guard.0.read(i)
    }

    pub fn write(&mut self, i: usize, value: u32) {
        assert!(i < NREGS, "регистра {i} нет");
        let (regs, journal) = &mut *self.guard;
        if journal.touched & (1 << i) == 0 {
            journal.touched |= 1 << i;
            journal.old[journal.len] = (i as u8, regs.read(i));
            journal.len += 1;
        }
        regs.write(i, value);
    }

    /// Подтвердить: изменения остаются, откат не выполняется.
    pub fn commit(self) {
        let _ = ScopeGuard::into_inner(self.guard);
    }
}
// ANCHOR_END: transaction
