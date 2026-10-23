//! Раздел 12.2: трейты Fn, FnMut и FnOnce, их иерархия и пересылка
//! через ссылку.

// ANCHOR: hierarchy
/// Вызывает `f` дважды и только читает её состояние: `&self`.
pub fn call_twice<F: Fn() -> u32>(f: F) -> u32 {
    f() + f()
}

/// Вызывает дважды и может менять состояние: `&mut self`.
pub fn call_twice_mut<F: FnMut() -> u32>(mut f: F) -> u32 {
    f() + f()
}

/// Вызывает один раз и может забрать состояние себе: `self`.
pub fn call_once<F: FnOnce() -> T, T>(f: F) -> T {
    f()
}
// ANCHOR_END: hierarchy

// ANCHOR: which
/// Какие трейты реализует замыкание, решает тело, а не `move`.
pub fn which() -> (u32, u32, u32, [u8; 4]) {
    let base = 20;
    let mut calls = 0;
    let reads = || base + 1; //                  Fn + FnMut + FnOnce
    let moved = move || base + 2; //             тоже Fn: только читает копию
    let counts = || {
        calls += 1; //                            FnMut + FnOnce
        calls
    };
    let id = *b"TC01";
    let gives = move || id; //                    Fn: [u8; 4] — Copy
    (call_twice(reads), call_twice(moved), call_twice_mut(counts), call_once(gives))
}
// ANCHOR_END: which

/// Замыкание, которое отдаёт захваченное значение не-`Copy`-типа, можно
/// вызвать один раз, и `FnMut` оно не реализует.
///
/// ```compile_fail,E0525
/// use ch12::traits::call_twice_mut;
/// let s = String::from("кадр");
/// let f = move || {
///     let owned = s;
///     owned.len() as u32
/// };
/// call_twice_mut(f);
/// ```
///
/// `FnMut` нельзя вызвать через неизменяемую привязку: вызов берёт
/// `&mut self`.
///
/// ```compile_fail,E0596
/// let mut count = 0;
/// let bump = || count += 1;
/// bump();
/// ```
///
/// А `FnOnce` нельзя вызвать дважды: первый вызов переместил замыкание.
///
/// ```compile_fail,E0382
/// let s = String::from("кадр");
/// let f = move || s;
/// let a = f();
/// let b = f();
/// ```
pub const fn errors() {}

// ANCHOR: forward
/// `&mut F` сам реализует `FnMut`, а `&F` — `Fn`: функция, которая берёт
/// замыкание по значению, получает ссылку, а замыкание остаётся у нас.
pub fn forward() -> u32 {
    let mut total = 0;
    let mut add = |x: u32| total += x;
    for_each_byte(b"\x01\x02", &mut add);
    for_each_byte(b"\x03", &mut add);
    add(4);
    total
}

pub fn for_each_byte<F: FnMut(u32)>(data: &[u8], mut f: F) {
    for &b in data {
        f(u32::from(b));
    }
}
// ANCHOR_END: forward

// ANCHOR: recursive
/// Рекурсивный обход с замыканием: `&mut dyn FnMut` вместо `F`, иначе
/// каждый уровень рекурсии требовал бы копию функции для типа
/// `&mut &mut … F`, и компилятор упёрся бы в предел глубины.
pub fn visit(depth: u32, f: &mut dyn FnMut(u32)) {
    f(depth);
    if depth > 0 {
        visit(depth - 1, f);
    }
}
// ANCHOR_END: recursive

/// Тот же обход с параметром типа: каждый вызов `visit_generic(…, &mut f)`
/// — новая копия функции для `&mut F`, `&mut &mut F` и так далее.
///
/// ```compile_fail
/// fn visit_generic<F: FnMut(u32)>(depth: u32, mut f: F) {
///     f(depth);
///     if depth > 0 {
///         visit_generic(depth - 1, &mut f);
///     }
/// }
/// visit_generic(3, |_| {});
/// ```
pub const fn recursion_limit() {}

// ANCHOR: once-drop
/// `FnOnce` забирает захваченное: если тело не отдаёт значение дальше,
/// оно уничтожается в конце вызова, а если замыкание так и не вызвали —
/// вместе с замыканием.
pub fn once_drop(log: &mut [u8; 4]) {
    struct Noisy<'a>(&'a mut u8);
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            *self.0 += 1;
        }
    }
    let [a, b, ..] = log;
    let called = {
        let n = Noisy(a);
        move || drop(n)
    };
    let never = {
        let n = Noisy(b);
        move || drop(n)
    };
    called(); //  a: деструктор отработал при вызове
    drop(never); // b: при уничтожении замыкания
}
// ANCHOR_END: once-drop

#[cfg(feature = "alloc")]
// ANCHOR: boxed-once
/// `Box<dyn FnOnce>` можно вызвать: тело перемещается из кучи, память
/// освобождается после вызова. Так устроены очереди отложенных действий.
pub fn deferred() -> alloc::vec::Vec<u32> {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    let mut out = Vec::new();
    let tasks: Vec<Box<dyn FnOnce() -> u32>> = {
        let label = alloc::string::String::from("три");
        alloc::vec![Box::new(|| 1), Box::new(move || label.len() as u32)]
    };
    for task in tasks {
        out.push(task());
    }
    out
}
// ANCHOR_END: boxed-once

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traits_by_body() {
        assert_eq!(which(), (42, 44, 3, *b"TC01"));
        assert_eq!(forward(), 10);
        let mut seen = 0u32;
        visit(3, &mut |d| seen = seen * 10 + d);
        assert_eq!(seen, 3210);
        let mut log = [0; 4];
        once_drop(&mut log);
        assert_eq!(log, [1, 1, 0, 0]);
        #[cfg(feature = "alloc")]
        assert_eq!(deferred(), [1, 6]);
    }
}
