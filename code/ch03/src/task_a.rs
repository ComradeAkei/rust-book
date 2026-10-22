//! Задача 3.A: восемь фрагментов, порядок уничтожения в которых нужно
//! предсказать. Каждый фрагмент — функция, события пишутся в журнал
//! `drop_order` текущего потока: имя из `Noisy` при уничтожении и строки
//! из `log`.

use crate::drop_order::{Noisy, log};
use std::vec;

/// Число фрагментов.
pub const COUNT: usize = 8;

// ANCHOR: list
// №1. Вложенные структуры, у обеих свой `Drop`.
pub struct Radio {
    pub phy: Noisy,
    pub mac: Mac,
}

pub struct Mac {
    pub tx: Noisy,
    pub rx: Noisy,
}

impl Drop for Radio {
    fn drop(&mut self) {
        log("Radio");
    }
}

impl Drop for Mac {
    fn drop(&mut self) {
        log("Mac");
    }
}

pub fn f1() {
    let _r = Radio { mac: Mac { rx: Noisy("rx"), tx: Noisy("tx") }, phy: Noisy("phy") };
}

// №2. Затенение и присваивание.
#[allow(unused_variables, unused_assignments)]
pub fn f2() {
    let a = Noisy("a1");
    let mut b = Noisy("b1");
    let a = Noisy("a2");
    b = Noisy("b2");
    log("конец");
    let _ = (&a, &b);
}

// №3. Временные значения и `let _` со значением.
pub fn f3() {
    let _ = Noisy("_");
    let _x = Noisy("_x");
    let len = Noisy("tmp").0.len();
    log("конец");
    let _ = len;
}

// №4. `let _` с переменной и с блоком.
pub fn f4() {
    let g = Noisy("g");
    let _ = g;
    log("после let _ = g");
    let h = Noisy("h");
    let _ = { h };
    log("конец");
}

// №5. Вектор, кортеж и частичное перемещение образцом.
pub fn f5() {
    let v = vec![Noisy("v0"), Noisy("v1"), Noisy("v2")];
    let t = (Noisy("t0"), Noisy("t1"));
    let (_a, _) = t;
    drop(v);
    log("конец");
}

// №6. Владение уходит в функцию и возвращается.
fn pass(n: Noisy) -> Noisy {
    log("pass");
    n
}

fn eat(_n: Noisy) {
    log("eat");
}

pub fn f6() {
    let a = Noisy("a");
    let _b = pass(a);
    eat(Noisy("c"));
    pass(Noisy("d"));
    log("конец");
}

// №7. Паника посреди создания массива.
fn explode(v: &[Noisy]) -> Noisy {
    panic!("в векторе {} элемента", v.len());
}

pub fn f7() {
    let _a = Noisy("a");
    let v = vec![Noisy("v0"), Noisy("v1")];
    let _b = Noisy("b");
    let _arr = [Noisy("x0"), explode(&v), Noisy("x2")];
}

// №8. `if let` с перемещением и `match` по временному значению.
#[allow(clippy::single_match)] // здесь нужен именно match, а не if let
pub fn f8() {
    let s = Some(Noisy("s"));
    if let Some(_n) = s {
        log("в ветке");
    }
    log("после if let");
    match Some(Noisy("m")) {
        Some(_) => log("ветка match"),
        None => {}
    }
    log("конец");
}
// ANCHOR_END: list

/// Фрагменты по порядку, для тестов.
pub const FRAGMENTS: [fn(); COUNT] = [f1, f2, f3, f4, f5, f6, f7, f8];
