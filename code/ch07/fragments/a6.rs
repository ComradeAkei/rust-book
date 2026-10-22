use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct SensorFault {
    pub channel: u8,
}

impl fmt::Display for SensorFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "датчик на канале {} не отвечает", self.channel)
    }
}

fn read(channel: u8) -> Result<i16, SensorFault> {
    if channel < 4 { Ok(channel as i16 * 10) } else { Err(SensorFault { channel }) }
}

pub fn average(channels: &[u8]) -> Result<i16, Box<dyn Error>> {
    let mut sum = 0i32;
    for &c in channels {
        sum += read(c)? as i32;
    }
    Ok((sum / channels.len().max(1) as i32) as i16)
}
