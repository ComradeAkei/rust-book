//! Раздел 7.1: `main`, который возвращает `Result`. При ошибке программа
//! печатает её через `Debug`, а не `Display`, и завершается с кодом 1.
//!
//! Запуск: `cargo run -p ch07 --bin main_result -- 115200`.

// ANCHOR: main
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let arg = std::env::args().nth(1).ok_or("нужна скорость порта")?;
    let baud: u32 = arg.parse()?;
    println!("скорость {baud}");
    Ok(())
}
// ANCHOR_END: main
