// Поля не помещаются в 32 бита — ошибка на ширине поля, которое вышло за край.
#[derive(tasks_macros::Bitfield)]
pub struct Wide {
    #[bits(16)]
    pub lo: u16,
    #[bits(12)]
    pub mid: u16,
    #[bits(8)]
    pub hi: u8,
}

fn main() {}
