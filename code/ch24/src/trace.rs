//! 24.3: гигиена в атрибутном макросе. `#[traced]` вставляет в начало тела
//! функции `let name = "имя_функции";` и отмечает вызов в журнале потока.
//! У `#[traced]` имя `name` создано со спаном `Span::mixed_site()`, у
//! `#[traced_call_site]` — со спаном `Span::call_site()`.

use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// Отметка о вызове. Её вставляет `#[traced]`.
pub fn enter(name: &'static str) {
    LOG.with_borrow_mut(|log| log.push(name));
}

/// Журнал текущего потока; после вызова он пуст.
pub fn take() -> Vec<&'static str> {
    LOG.with_borrow_mut(std::mem::take)
}

// ANCHOR: greet
/// Длина имени. Аргумент `name` совпадает с именем переменной макроса.
#[crate::traced]
pub fn greet(name: &str) -> usize {
    name.len()
}

/// То же с `#[traced_call_site]`: переменная макроса перекрывает аргумент,
/// и тело считает длину строки `"greet_call_site"`. Компилятор замечает
/// только неиспользованный аргумент.
#[crate::traced_call_site]
#[expect(unused_variables)]
pub fn greet_call_site(name: &str) -> usize {
    name.len()
}
// ANCHOR_END: greet

#[cfg(test)]
mod tests {
    use super::*;

    // ANCHOR: greet_test
    #[test]
    fn mixed_site_keeps_argument() {
        assert_eq!(greet("Ivan"), 4);
        assert_eq!(greet_call_site("Ivan"), "greet_call_site".len());
        assert_eq!(take(), ["greet", "greet_call_site"]);
    }
    // ANCHOR_END: greet_test
}
