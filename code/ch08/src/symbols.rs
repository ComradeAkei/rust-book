//! Раздел 8.7: во что превращаются имена функций в таблице символов.
use core::hint::black_box;

// ANCHOR: items
pub struct Uart {
    pub sent: u32,
}

impl Uart {
    #[inline(never)]
    pub fn send(&mut self, b: u8) -> u8 {
        self.sent += 1;
        black_box(b)
    }
}

#[inline(never)]
pub fn checksum<T: AsRef<[u8]>>(data: T) -> u8 {
    data.as_ref().iter().fold(0u8, |a, &b| a.wrapping_add(b))
}

/// Две конкретизации обобщённой функции — два символа.
pub fn both(frame: &[u8], header: [u8; 4]) -> (u8, u8) {
    (checksum(frame), checksum(header))
}

#[unsafe(no_mangle)]
pub extern "C" fn station_reset() -> u32 {
    0
}

#[unsafe(export_name = "station_version")]
pub extern "C" fn version() -> u32 {
    0x0001_0800
}
// ANCHOR_END: items

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        let mut u = Uart { sent: 0 };
        assert_eq!(u.send(5), 5);
        assert_eq!(u.sent, 1);
        assert_eq!(both(&[1, 2, 3], [250, 5, 1, 0]), (6, 0));
        assert_eq!(station_reset(), 0);
        assert_eq!(version(), 0x0001_0800);
    }
}
