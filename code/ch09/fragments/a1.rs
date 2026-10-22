// Задача 9.A, фрагмент a1. Отдельный крейт: всё из core и std для него чужое.
use std::fmt;

pub struct Reading(pub i32);

pub trait Describe {
    fn describe(&self) -> String;
}

// ⟨1⟩
impl fmt::Display for Vec<Reading> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} отсчётов", self.len())
    }
}

// ⟨2⟩
impl From<Reading> for Vec<u8> {
    fn from(r: Reading) -> Vec<u8> {
        r.0.to_le_bytes().to_vec()
    }
}
