use std::array::TryFromSliceError;
use std::error::Error;

type BoxError = Box<dyn Error + Send + Sync>;

fn header(frame: &[u8]) -> Result<[u8; 4], TryFromSliceError> {
    frame.get(..4).unwrap_or(&[]).try_into()
}

pub fn kind(frame: &[u8]) -> Result<u8, BoxError> {
    let h = header(frame)?;
    if h[0] != 0xA5 {
        Err(format!("неверный маркер {:#04x}", h[0]))?;
    }
    Ok(h[2])
}
