//! Раздел 7.4: размер типа ошибки — это размер каждого `Result`, который
//! функция возвращает, и на пути успеха тоже.

use alloc::boxed::Box;

// ANCHOR: fat
/// Подробная ошибка разбора: копия плохой строки прямо в ошибке.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FatError {
    pub line: u32,
    pub col: u32,
    pub text: [u8; 64],
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_parse_fat(b: &[u8]) -> Result<u32, FatError> {
    match b.first() {
        Some(&d @ b'0'..=b'9') => Ok((d - b'0') as u32),
        _ => {
            let mut text = [0; 64];
            let n = b.len().min(64);
            text[..n].copy_from_slice(&b[..n]);
            Err(FatError { line: 1, col: 1, text })
        }
    }
}

/// Та же ошибка в куче: `Result` занимает 16 байт, а не 76.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_parse_boxed(b: &[u8]) -> Result<u32, Box<FatError>> {
    match b.first() {
        Some(&d @ b'0'..=b'9') => Ok((d - b'0') as u32),
        _ => {
            let mut e = Box::new(FatError { line: 1, col: 1, text: [0; 64] });
            let n = b.len().min(64);
            e.text[..n].copy_from_slice(&b[..n]);
            Err(e)
        }
    }
}

/// Уровень выше: `?` передаёт ошибку дальше.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_outer_fat(b: &[u8]) -> Result<u32, FatError> {
    Ok(ch07_parse_fat(b)? * 10)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch07_outer_boxed(b: &[u8]) -> Result<u32, Box<FatError>> {
    Ok(ch07_parse_boxed(b)? * 10)
}
// ANCHOR_END: fat

// ANCHOR: non_exhaustive
/// Ошибки драйвера, которые могут пополниться в следующих версиях крейта.
/// Вне крейта `match` по ним обязан иметь ветку `_`:
///
/// ```compile_fail,E0004
/// use ch07::sizes::DriverError;
/// fn retry(e: DriverError) -> bool {
///     match e {
///         DriverError::Timeout => true,
///         DriverError::Crc => false,
///     }
/// }
/// ```
///
/// ```
/// use ch07::sizes::DriverError;
/// fn retry(e: DriverError) -> bool {
///     match e {
///         DriverError::Timeout => true,
///         _ => false,
///     }
/// }
/// assert!(retry(DriverError::Timeout));
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverError {
    Timeout,
    Crc,
}
// ANCHOR_END: non_exhaustive

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn sizes_of_results() {
        assert_eq!(size_of::<FatError>(), 72);
        assert_eq!(size_of::<Result<u32, FatError>>(), 76);
        assert_eq!(size_of::<Result<u32, Box<FatError>>>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<Result<(), Box<FatError>>>(), size_of::<usize>());
    }

    #[test]
    fn same_answers() {
        assert_eq!(ch07_parse_fat(b"7"), Ok(7));
        assert_eq!(ch07_parse_boxed(b"7"), Ok(7));
        assert_eq!(ch07_outer_fat(b"7"), Ok(70));
        assert_eq!(ch07_outer_boxed(b"7"), Ok(70));
        let e = ch07_outer_boxed(b"x=1").unwrap_err();
        assert_eq!(&e.text[..3], b"x=1");
        assert_eq!(ch07_outer_fat(b"x=1").unwrap_err(), *e);
    }
}
