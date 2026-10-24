//! Задача A: где живут десять типов и функций.

/// Слой стандартной библиотеки, в котором есть элемент.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// Только `core`: работает везде, памяти и ОС не нужно.
    Core,
    /// Нужен `alloc`: куча, но не ОС.
    Alloc,
    /// Только `std`: нужна операционная система.
    Std,
    /// Ответа ещё нет.
    Unknown,
}

/// Можно ли пользоваться элементом в `no_std`-крейте для
/// thumbv6m-none-eabi (Cortex-M0), подключив `alloc`, если нужно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnM0 {
    Yes,
    No,
    Unknown,
}

// ANCHOR: answers
pub const ANSWERS: [(&str, Home, OnM0); 10] = [
    ("Duration", Home::Core, OnM0::Yes),
    ("Instant", Home::Std, OnM0::No),
    ("<[T]>::sort", Home::Alloc, OnM0::Yes),
    ("<[T]>::sort_unstable", Home::Core, OnM0::Yes),
    ("f32::sqrt", Home::Std, OnM0::No),
    ("f32::abs", Home::Core, OnM0::Yes),
    ("HashMap", Home::Std, OnM0::No),
    ("BTreeMap", Home::Alloc, OnM0::Yes),
    ("Arc", Home::Alloc, OnM0::No),
    ("AtomicU32::fetch_add", Home::Core, OnM0::No),
];
// ANCHOR_END: answers
