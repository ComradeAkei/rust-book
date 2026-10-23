use std::io;

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Range(u32),
}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError::Io(e)
    }
}

pub fn baud(path: &str) -> Result<u32, ConfigError> {
    let text = std::fs::read_to_string(path)?;
    let b: u32 = text.trim().parse()?;
    if b > 1_000_000 {
        return Err(ConfigError::Range(b));
    }
    Ok(b)
}
