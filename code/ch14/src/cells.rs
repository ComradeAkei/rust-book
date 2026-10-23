//! Раздел 14.5: внутренняя изменяемость.
//!
//! `UnsafeCell` — единственный законный способ менять значение через общую
//! ссылку; свой `Cell` поверх него; счётчики порта в `Cell`; реестр
//! подписчиков из `kref.c` на `RefCell`, который паникует там, где C
//! портил память, и его исправленный вариант.

use core::cell::{Cell, RefCell, UnsafeCell};

// ANCHOR: mycell
/// `Cell` в миниатюре. Значение лежит в `UnsafeCell`, и только поэтому
/// его можно менять через `&self`: для всего остального компилятор
/// считает, что за общей ссылкой ничего не меняется.
pub struct MyCell<T> {
    value: UnsafeCell<T>,
}

impl<T: Copy> MyCell<T> {
    pub const fn new(value: T) -> Self {
        Self { value: UnsafeCell::new(value) }
    }

    pub fn get(&self) -> T {
        // SAFETY: ссылок внутрь значения MyCell не выдаёт, а `UnsafeCell`
        // делает тип `!Sync`: одновременно с этим чтением записи нет.
        unsafe { *self.value.get() }
    }

    pub fn set(&self, value: T) {
        // SAFETY: то же — запись не пересекается ни с какой ссылкой внутрь.
        unsafe { *self.value.get() = value }
    }
}
// ANCHOR_END: mycell

/// `MyCell` не `Sync` — его нельзя положить в `static` и делить между
/// потоками, как и `Cell`:
///
/// ```compile_fail,E0277
/// static HITS: ch14::cells::MyCell<u32> = ch14::cells::MyCell::new(0);
/// ```
///
/// Запись через общую ссылку без `UnsafeCell` компилятор отвергает сам,
/// линтом `invalid_reference_casting`, который запрещён по умолчанию:
///
/// ```compile_fail
/// fn bump(x: &u32) {
///     unsafe { *(x as *const u32 as *mut u32) += 1 };
/// }
/// ```
// ANCHOR: port
/// Порт, общий для нескольких задач одного потока: `send` принимает
/// `&self`, а счётчики меняются через `Cell`. `Cell` не выдаёт ссылок
/// внутрь — только копирует значение наружу и внутрь.
pub struct Port {
    sent: Cell<u32>,
    errors: Cell<u32>,
}

impl Port {
    pub const fn new() -> Self {
        Self { sent: Cell::new(0), errors: Cell::new(0) }
    }

    pub fn send(&self, frame: &[u8]) -> bool {
        let ok = !frame.is_empty();
        let counter = if ok { &self.sent } else { &self.errors };
        counter.set(counter.get() + 1);
        ok
    }

    /// Счётчики за период: прочесть и обнулить одной операцией.
    pub fn take_stats(&self) -> (u32, u32) {
        (self.sent.take(), self.errors.take())
    }
}
// ANCHOR_END: port

impl Default for Port {
    fn default() -> Self {
        Self::new()
    }
}

// ANCHOR: registry
pub type Notify = fn(&Registry, u32);

/// Реестр подписчиков из `kref.c`: список в `RefCell`, потому что
/// подписываться можно через общую ссылку, в том числе из обработчика.
pub struct Registry {
    subs: RefCell<[Option<(Notify, u32)>; 4]>,
}

impl Registry {
    pub const fn new() -> Self {
        Self { subs: RefCell::new([None; 4]) }
    }

    pub fn subscribe(&self, f: Notify) {
        let mut subs = self.subs.borrow_mut(); // паника, если список читают
        let free = subs.iter_mut().find(|s| s.is_none()).expect("мест нет");
        *free = Some((f, 0));
    }

    /// Как `config_set_period`: обходит список и зовёт каждого.
    pub fn set_period(&self, ms: u32) {
        for slot in self.subs.borrow_mut().iter_mut().flatten() {
            (slot.0)(self, ms); // обработчик может позвать subscribe
            slot.1 += 1;
        }
    }

    /// Исправление: заимствование живёт один шаг и отпущено на время
    /// вызова. Подписчик, добавленный в обходе, тоже получит вызов — как
    /// в C, где `nsubs` перечитывается на каждом шаге.
    pub fn set_period_fixed(&self, ms: u32) {
        let n = self.subs.borrow().len(); // не в заголовке for: раздел 14.8
        for i in 0..n {
            let Some((f, _)) = self.subs.borrow()[i] else { continue };
            f(self, ms);
            if let Some(slot) = &mut self.subs.borrow_mut()[i] {
                slot.1 += 1;
            }
        }
    }

    pub fn calls(&self) -> [u32; 4] {
        self.subs.borrow().map(|s| s.map_or(0, |s| s.1))
    }
}
// ANCHOR_END: registry

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

// ANCHOR: watchdog
pub fn on_change(_: &Registry, _: u32) {}

/// Сторож: если период вырос больше секунды, подписывает тревогу.
pub fn watchdog(r: &Registry, ms: u32) {
    if ms > 1000 {
        r.subscribe(on_change);
    }
}
// ANCHOR_END: watchdog

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mycell_like_cell() {
        let c = MyCell::new(1u32);
        let r1 = &c;
        let r2 = &c;
        r1.set(r2.get() + 1);
        assert_eq!(c.get(), 2);
    }

    #[test]
    fn port_counts_through_shared_ref() {
        let port = Port::new();
        let (a, b) = (&port, &port);
        assert!(a.send(b"\xa5"));
        assert!(!b.send(b""));
        assert!(a.send(b"\x01"));
        assert_eq!(port.take_stats(), (2, 1));
        assert_eq!(port.take_stats(), (0, 0));
    }

    #[test]
    #[should_panic(expected = "RefCell already borrowed")]
    fn registry_panics_where_c_corrupted_memory() {
        let r = Registry::new();
        r.subscribe(on_change);
        r.subscribe(watchdog);
        r.set_period(2000);
    }

    #[test]
    fn registry_fixed_notifies_all() {
        let r = Registry::new();
        r.subscribe(on_change);
        r.subscribe(watchdog);
        r.set_period_fixed(2000);
        assert_eq!(r.calls(), [1, 1, 1, 0]);
        r.set_period_fixed(500);
        assert_eq!(r.calls(), [2, 2, 2, 0]);
    }

    #[test]
    fn borrow_states() {
        let c = RefCell::new(5);
        let a = c.borrow();
        let b = c.borrow();
        assert!(c.try_borrow_mut().is_err(), "читатели есть — писать нельзя");
        drop((a, b));
        let mut m = c.borrow_mut();
        *m += 1;
        assert!(c.try_borrow().is_err(), "писатель есть — читать нельзя");
        drop(m);
        assert_eq!(*c.borrow(), 6);
    }

    // Наблюдение rustc 1.97: флаг заимствования RefCell — isize перед
    // значением, Cell не добавляет ничего.
    #[test]
    fn observed_cell_sizes() {
        use core::mem::size_of;
        let w = size_of::<usize>();
        assert_eq!(size_of::<Cell<u32>>(), 4);
        assert_eq!(size_of::<UnsafeCell<u32>>(), 4);
        assert_eq!(size_of::<RefCell<u32>>(), 2 * w);
        assert_eq!(size_of::<RefCell<u8>>(), 2 * w);
        assert_eq!(size_of::<RefCell<[u64; 2]>>(), w + 16);
    }
}
