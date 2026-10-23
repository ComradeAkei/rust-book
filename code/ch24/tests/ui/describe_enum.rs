// Derive сам отвергает перечисление и указывает на `enum`.
#[derive(ch24::Describe)]
pub enum Mode {
    Off,
    On,
}

fn main() {}
