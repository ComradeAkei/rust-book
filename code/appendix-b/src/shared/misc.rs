//! Разделы B.4 и B.5: изменения, которые видны на одном исходнике в обеих
//! редакциях. Файл компилируется дважды, как `temporaries.rs`.

// ANCHOR: boxed_slice
/// Тип элемента, который выдаёт `into_iter()` у `Box<[u32]>`.
pub fn boxed_slice_item(b: Box<[u32]>) -> &'static str {
    fn type_of<T>(_: &T) -> &'static str {
        core::any::type_name::<T>()
    }
    let first = b.into_iter().next();
    type_of(&first)
}
// ANCHOR_END: boxed_slice

// ANCHOR: expr_fragment
macro_rules! classify {
    ($e:expr) => {
        "expr"
    };
    (const $b:block) => {
        "const-блок"
    };
    (_) => {
        "подчёркивание"
    };
}

/// Какое правило макроса сработает для `const { ... }`.
pub fn classify_const_block() -> &'static str {
    classify!(const { 1 + 1 })
}

/// И для `_`.
pub fn classify_underscore() -> &'static str {
    classify!(_)
}
// ANCHOR_END: expr_fragment
