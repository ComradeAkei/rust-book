// Задача 9.A, фрагмент a6. Отдельный крейт: всё из core и std для него чужое.
use std::fmt;

pub struct Reading(pub i32);

pub trait Describe {
    fn describe(&self) -> String;
}

// ⟨1⟩
impl std::ops::Add<i32> for Reading {
    type Output = Reading;

    fn add(self, rhs: i32) -> Reading {
        Reading(self.0 + rhs)
    }
}

// ⟨2⟩
impl<T> std::ops::Add<Reading> for T {
    type Output = Reading;

    fn add(self, rhs: Reading) -> Reading {
        rhs
    }
}
