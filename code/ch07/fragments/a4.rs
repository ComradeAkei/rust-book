use std::io;

#[derive(Debug)]
pub struct ConfigError(pub io::Error);

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError(e)
    }
}

#[derive(Debug)]
pub enum AppError {
    Config(ConfigError),
    Sensor(u8),
}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::Config(e)
    }
}

fn load(path: &str) -> Result<String, io::Error> {
    std::fs::read_to_string(path)
}

pub fn start(path: &str) -> Result<usize, AppError> {
    let text = load(path)?;
    if text.is_empty() {
        return Err(AppError::Sensor(0));
    }
    Ok(text.len())
}
