// Обещанный размер не совпал: E0080 указывает на литерал в атрибуте.
#[derive(ch24::Describe)]
#[describe(size = 8)]
#[repr(C)]
pub struct Frame {
    pub id: u32,
    pub len: u32,
    pub crc: u32,
}

fn main() {}
