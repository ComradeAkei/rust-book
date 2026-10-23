//! Задача B: реестр обработчиков событий без кучи, двумя способами.
//!
//! `Registry` хранит заимствованные `&mut dyn FnMut(Event)`, `Chain` — сами
//! замыкания в виде вложенных пар. Типы и сигнатуры даны, тела — `todo!()`.
//! Кто владеет состоянием обработчиков в каждом случае и что из этого
//! следует для вызывающего, объясните в комментарии к решению.

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
        todo!()
    }

    /// Номер ячейки, которую занял обработчик.
    pub fn subscribe(&mut self, h: &'a mut dyn FnMut(Event)) -> Result<usize, Full> {
        todo!()
    }

    /// Освободить ячейку и вернуть заём тому, кто его дал.
    pub fn unsubscribe(&mut self, id: usize) -> Option<&'a mut dyn FnMut(Event)> {
        todo!()
    }

    /// Вызвать обработчики по порядку ячеек; сколько их было.
    pub fn emit(&mut self, e: Event) -> usize {
        todo!()
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
        todo!()
    }
}

/// Сначала старшие обработчики, затем последний добавленный.
impl<T: Handlers, H: FnMut(Event)> Handlers for (T, H) {
    fn emit(&mut self, e: Event) -> usize {
        todo!()
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
        todo!()
    }

    pub fn emit(&mut self, e: Event) -> usize {
        todo!()
    }

    /// Забрать обработчики обратно, а с ними и их состояние.
    pub fn into_handlers(self) -> L {
        todo!()
    }
}
// ANCHOR_END: chain
