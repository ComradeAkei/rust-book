//! Задача B: реестр обработчиков событий без кучи, двумя способами.
//!
//! `Registry` хранит заимствованные `&mut dyn FnMut(Event)`: состояние
//! обработчиков живёт у вызывающего, реестр только держит заём. `Chain`
//! хранит сами замыкания: состояние переезжает в реестр, и тип реестра
//! растёт с каждым обработчиком.

// ANCHOR: event
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    Button(bool),
    Tick(u32),
    Sample(i16),
}

/// В реестре нет свободного места.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Full;
// ANCHOR_END: event

// ANCHOR: registry
/// До `N` обработчиков, взятых взаймы на время `'a`.
pub struct Registry<'a, const N: usize> {
    slots: [Option<&'a mut dyn FnMut(Event)>; N],
}

impl<'a, const N: usize> Registry<'a, N> {
    pub const fn new() -> Self {
        Registry { slots: [const { None }; N] }
    }

    /// Номер ячейки, которую занял обработчик.
    pub fn subscribe(&mut self, h: &'a mut dyn FnMut(Event)) -> Result<usize, Full> {
        let (i, slot) = self.slots.iter_mut().enumerate().find(|(_, s)| s.is_none()).ok_or(Full)?;
        *slot = Some(h);
        Ok(i)
    }

    /// Освободить ячейку и вернуть заём тому, кто его дал.
    pub fn unsubscribe(&mut self, id: usize) -> Option<&'a mut dyn FnMut(Event)> {
        self.slots.get_mut(id)?.take()
    }

    /// Вызвать обработчики по порядку ячеек; сколько их было.
    pub fn emit(&mut self, e: Event) -> usize {
        let mut n = 0;
        for h in self.slots.iter_mut().flatten() {
            h(e);
            n += 1;
        }
        n
    }
}

impl<const N: usize> Default for Registry<'_, N> {
    fn default() -> Self {
        Self::new()
    }
}
// ANCHOR_END: registry

// ANCHOR: chain
/// Список обработчиков как вложенные пары: `((((), A), B), C)`.
pub trait Handlers {
    fn emit(&mut self, e: Event) -> usize;
}

impl Handlers for () {
    fn emit(&mut self, _: Event) -> usize {
        0
    }
}

/// Сначала старшие обработчики, затем последний добавленный.
impl<T: Handlers, H: FnMut(Event)> Handlers for (T, H) {
    fn emit(&mut self, e: Event) -> usize {
        let n = self.0.emit(e);
        (self.1)(e);
        n + 1
    }
}

/// Реестр, который владеет обработчиками. Каждый `with` возвращает реестр
/// нового типа.
pub struct Chain<L> {
    list: L,
}

impl Chain<()> {
    pub const fn new() -> Self {
        Chain { list: () }
    }
}

impl Default for Chain<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Handlers> Chain<L> {
    pub fn with<H: FnMut(Event)>(self, h: H) -> Chain<(L, H)> {
        Chain { list: (self.list, h) }
    }

    pub fn emit(&mut self, e: Event) -> usize {
        self.list.emit(e)
    }

    /// Забрать обработчики обратно, а с ними и их состояние.
    pub fn into_handlers(self) -> L {
        self.list
    }
}
// ANCHOR_END: chain

// ANCHOR: probe
/// Для ассемблера в разборе: два обработчика в `Chain` и в `Registry` на
/// одном и том же потоке событий.
#[unsafe(no_mangle)]
pub fn ch12_b_chain(events: &[Event]) -> (u32, u32) {
    let (mut presses, mut last) = (0u32, 0u32);
    {
        let mut chain =
            Chain::new().with(|e| presses += u32::from(e == Event::Button(true))).with(|e| {
                if let Event::Tick(t) = e {
                    last = t;
                }
            });
        for &e in events {
            chain.emit(e);
        }
    }
    (presses, last)
}

#[unsafe(no_mangle)]
pub fn ch12_b_registry(events: &[Event]) -> (u32, u32) {
    let (mut presses, mut last) = (0u32, 0u32);
    let mut on_button = |e| presses += u32::from(e == Event::Button(true));
    let mut on_tick = |e| {
        if let Event::Tick(t) = e {
            last = t;
        }
    };
    {
        let mut r = Registry::<'_, 2>::new();
        let _ = r.subscribe(&mut on_button);
        let _ = r.subscribe(&mut on_tick);
        for &e in events {
            r.emit(e);
        }
    }
    (presses, last)
}

/// `emit` реестра, который заполнили где-то ещё: что лежит в ячейках,
/// здесь не видно.
#[unsafe(no_mangle)]
pub fn ch12_b_emit(r: &mut Registry<'_, 4>, e: Event) -> usize {
    r.emit(e)
}
// ANCHOR_END: probe

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_agree() {
        let events = [Event::Button(true), Event::Tick(4), Event::Button(false), Event::Tick(9)];
        assert_eq!(ch12_b_chain(&events), (1, 9));
        assert_eq!(ch12_b_registry(&events), (1, 9));
    }
}
