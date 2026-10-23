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
// TODO(задача A): замените Unknown своими ответами. Сначала ответьте, потом
// запускайте тест: он собирает пробы закреплённым rustc и сверяет.
pub const ANSWERS: [(&str, Home, OnM0); 10] = [
    ("Duration", Home::Unknown, OnM0::Unknown),
    ("Instant", Home::Unknown, OnM0::Unknown),
    ("<[T]>::sort", Home::Unknown, OnM0::Unknown),
    ("<[T]>::sort_unstable", Home::Unknown, OnM0::Unknown),
    ("f32::sqrt", Home::Unknown, OnM0::Unknown),
    ("f32::abs", Home::Unknown, OnM0::Unknown),
    ("HashMap", Home::Unknown, OnM0::Unknown),
    ("BTreeMap", Home::Unknown, OnM0::Unknown),
    ("Arc", Home::Unknown, OnM0::Unknown),
    ("AtomicU32::fetch_add", Home::Unknown, OnM0::Unknown),
];
// ANCHOR_END: answers
