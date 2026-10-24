//! Помощники интеграционных тестов. Файл лежит в `tests/common/mod.rs`, а
//! не в `tests/common.rs`: каждый файл прямо в `tests/` cargo собирает
//! отдельным тестовым бинарником, подкаталог — нет.

use ch31::basics::Window;

// ANCHOR: common
/// Окно, в которое по порядку положены `xs`.
pub fn window_of<const N: usize>(xs: &[i32]) -> Window<N> {
    let mut w = Window::new();
    xs.iter().for_each(|&x| w.push(x));
    w
}
// ANCHOR_END: common
