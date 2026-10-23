//! Задача A: размеры замыканий.
//!
//! Ответ — функция от размера указателя `ptr`: тест сверяет `predicted(8)`
//! на x86-64, а `predicted(4)` — при сборке для Cortex-M3. Разбор каждого
//! замыкания — в приложении E.

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
    [
        0,       // c0: ничего не захвачено
        ptr,     // c1: `x` только читается — захват по ссылке, хоть он и Copy
        1,       // c2: копия u8
        16,      // c3: u8 и u64, выравнивание u64 — 8 и на Cortex-M
        2 * ptr, // c4: точный захват `*s` — ссылка на str, толстая
        ptr,     // c5: ссылка на одно поле r.value
        8,       // c6: копии r.value и r.id, без r.label
        1,       // c7: внутреннее замыкание по значению, а в нём u8
    ]
}
// ANCHOR_END: predicted
