// Задача 9.A, фрагмент a2. Отдельный крейт: всё из core и std для него чужое.
use std::fmt;

pub struct Reading(pub i32);

pub trait Describe {
    fn describe(&self) -> String;
}

// ⟨1⟩
impl<T: fmt::Display> Describe for T {
    fn describe(&self) -> String {
        format!("«{self}»")
    }
}

// ⟨2⟩
impl Describe for Reading {
    fn describe(&self) -> String {
        format!("отсчёт {}", self.0)
    }
}
