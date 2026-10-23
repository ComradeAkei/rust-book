use std::error::Error;
use std::fs;

pub fn rate(path: &str) -> Result<u32, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let line = text.lines().next().ok_or("пустой файл")?;
    let n: u32 = line.trim().parse()?;
    Ok(n)
}
