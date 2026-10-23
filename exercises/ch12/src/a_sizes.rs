//! Задача A: размеры замыканий.
//!
//! Сначала предскажите ответы без компилятора, затем проверьте: `measured`
//! меряет настоящие размеры. Ответ — функция от размера указателя `ptr`:
//! тест сверяет `predicted(8)` на x86-64, а `predicted(4)` — при сборке для
//! Cortex-M3, где `measured` вычисляет компилятор цели.

// ANCHOR: closures
pub struct Reading {
    pub id: u16,
    pub value: i32,
    pub label: &'static str,
}

/// Восемь замыканий и `size_of_val` каждого. Вызывать их незачем: размер
/// известен, как только замыкание создано.
pub const fn measured() -> [usize; 8] {
    let x = 7u8;
    let (a, b) = (1u8, 2u64);
    let s: &str = "кадр";
    let r = Reading { id: 3, value: -40, label: "t" };
    let c0 = || 7u8;
    let c1 = || x;
    let c2 = move || x;
    let c3 = move || (a, b);
    let c4 = || s.len();
    let c5 = || r.value + 1;
    let c6 = move || r.value + r.id as i32;
    let inner = move || x;
    let c7 = move || inner() + 1;
    [
        size_of_val(&c0),
        size_of_val(&c1),
        size_of_val(&c2),
        size_of_val(&c3),
        size_of_val(&c4),
        size_of_val(&c5),
        size_of_val(&c6),
        size_of_val(&c7),
    ]
}
// ANCHOR_END: closures

// ANCHOR: predicted
/// Размеры замыканий `c0`–`c7` в байтах для цели, где указатель занимает
/// `ptr` байт (8 или 4).
pub const fn predicted(ptr: usize) -> [usize; 8] {
    panic!("предскажите без компилятора, затем сверьте с measured()")
}
// ANCHOR_END: predicted
