// Поле без Debug: спан сгенерированного вызова взят у типа поля.
pub struct Opaque;

#[derive(ch24::Describe)]
pub struct Holder {
    pub id: u8,
    pub raw: Opaque,
}

fn main() {}
