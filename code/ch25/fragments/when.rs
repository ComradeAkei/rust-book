// Раздел 25.3: когда rustc вычисляет константу. Тест tests/eval.rs
// собирает этот файл с `--emit metadata` (так работает `cargo check`) и
// с `--emit link` (так работает `cargo build`), по одному случаю за раз
// (`--cfg case="…"`), и сравнивает, есть ли ошибка E0080.

// ANCHOR: when
const fn small(x: u32) -> u32 {
    assert!(x < 10, "small: нужно меньше 10");
    x
}

// Свободная константа: вычисляется всегда, и при check, и при build.
#[cfg(case = "item")]
pub const LIMIT: u32 = small(42);

// Блок const в обычной функции: rustc 1.97 вычисляет его при сборке, даже
// в ветке, которая не исполняется, — но не при check.
#[cfg(case = "block")]
pub fn limit() -> u32 {
    if false { const { small(42) } } else { 0 }
}

// В обобщённой функции — при мономорфизации: нет вызова, нет и ошибки.
#[cfg(case = "generic")]
pub fn limit_for<T>() -> u32 {
    const { small(42) }
}
// ANCHOR_END: when
