// Ширина больше типа поля и bool не в один бит — ошибки на литералах.
#[derive(tasks_macros::Bitfield)]
pub struct Ifls {
    #[bits(9)]
    pub txiflsel: u8,
    #[bits(2)]
    pub enabled: bool,
    #[bits(0)]
    pub empty: u16,
}

fn main() {}
