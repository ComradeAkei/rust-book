// Три ошибки в атрибутах-помощниках — все сразу, каждая со своим спаном.
#[derive(ch24::Describe)]
#[describe(sise = 8)]
#[describe(size = "8")]
pub struct Frame {
    pub id: u32,
    #[describe(skip, hex)]
    pub crc: u32,
}

fn main() {}
