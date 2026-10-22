// Задача 9.A, фрагмент a5. Отдельный крейт: всё из core и std для него чужое.
use std::fmt;

pub struct Reading(pub i32);

pub trait Describe {
    fn describe(&self) -> String;
}

// ⟨1⟩
impl<T: Into<i32>> From<T> for Reading {
    fn from(v: T) -> Reading {
        Reading(v.into())
    }
}

// ⟨2⟩
impl From<Reading> for i32 {
    fn from(r: Reading) -> i32 {
        r.0
    }
}
