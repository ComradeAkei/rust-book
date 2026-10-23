// Задача 9.A, фрагмент a4. Отдельный крейт: всё из core и std для него чужое.
// ANCHOR: header
use std::fmt;

pub struct Reading(pub i32);

pub trait Describe {
    fn describe(&self) -> String;
}
// ANCHOR_END: header

// ANCHOR: impls
// ⟨1⟩
impl fmt::Display for Box<Reading> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "отсчёт {} в куче", self.0)
    }
}

// ⟨2⟩
impl fmt::Display for &Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "отсчёт {} по ссылке", self.0)
    }
}
// ANCHOR_END: impls
