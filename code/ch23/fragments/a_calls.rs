// Задача 23.A: шесть вызовов макросов. Предскажите раскрытие каждого —
// тело `call1`…`call5` и блок `impl Point` после `getters!` — и сверьте:
// RUSTC_BOOTSTRAP=1 rustc --edition 2024 --crate-type lib -Zunpretty=expanded a_calls.rs
// (`cargo expand` печатает то же, отформатировав). Файл подключается и как
// модуль: общие тесты проверяют и поведение вызовов.

macro_rules! max {
    ($a:expr, $b:expr) => {
        if $a > $b { $a } else { $b }
    };
}

macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

macro_rules! square_tt {
    ($($x:tt)*) => {
        $($x)* * $($x)*
    };
}

macro_rules! swap {
    ($a:ident, $b:ident) => {
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}

macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! is_zero {
    (0) => {
        "ноль"
    };
    ($($t:tt)*) => {
        "не ноль"
    };
}

macro_rules! via_expr {
    ($e:expr) => {
        is_zero!($e)
    };
}

macro_rules! getters {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        impl $name {
            $(pub fn $field(&self) -> $ty { self.$field })*
        }
    };
}

pub fn call1(next: &mut impl FnMut() -> i32) -> i32 {
    max!(next(), 3)
}

pub fn call2() -> (i32, i32) {
    (square!(1 + 2), square_tt!(1 + 2))
}

pub fn call3(mut x: i32, mut tmp: i32) -> (i32, i32) {
    swap!(x, tmp);
    (x, tmp)
}

pub fn call4() -> usize {
    count!(a b c)
}

pub fn call5() -> (&'static str, &'static str) {
    (is_zero!(0), via_expr!(0))
}

pub struct Point {
    pub x: i32,
    pub y: i32,
}

getters!(Point { x: i32, y: i32 });
