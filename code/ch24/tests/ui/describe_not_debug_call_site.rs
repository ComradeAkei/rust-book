// То же поле без Debug, но весь сгенерированный код — со спаном вызова.
pub struct Opaque;

#[derive(ch24::DescribeCallSite)]
pub struct Holder {
    pub id: u8,
    pub raw: Opaque,
}

fn main() {}
