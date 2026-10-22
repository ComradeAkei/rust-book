// Задача 9.A, фрагмент a3. Отдельный крейт: всё из core и std для него чужое.
use std::fmt;

pub struct Reading(pub i32);

pub trait Describe {
    fn describe(&self) -> String;
}

// ⟨1⟩
impl<T: Copy> Describe for T {
    fn describe(&self) -> String {
        String::from("копируемое значение")
    }
}

// ⟨2⟩
impl Describe for Box<u8> {
    fn describe(&self) -> String {
        format!("байт в куче: {}", **self)
    }
}
