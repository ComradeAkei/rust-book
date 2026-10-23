// Тип поля, которого derive не умеет упаковать.
#[derive(tasks_macros::Bitfield)]
pub struct Sample {
    #[bits(8)]
    pub level: i8,
    #[bits(4)]
    pub flags: [bool; 4],
}

fn main() {}
