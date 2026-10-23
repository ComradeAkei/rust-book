// Все нарушения сигнатуры обработчика — за одну сборку.
use ch24::device::interrupt;

#[interrupt]
pub async fn UART1(byte: u8) -> u8 {
    byte
}

fn main() {}
