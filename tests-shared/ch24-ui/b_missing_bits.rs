// Поле без #[bits(N)] — ошибка на имени поля.
#[derive(tasks_macros::Bitfield)]
pub struct Ctl {
    #[bits(1)]
    pub uarten: bool,
    pub siren: bool,
}

fn main() {}
