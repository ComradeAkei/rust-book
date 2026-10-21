//! Раздел 2.2: циклы, `break` со значением, метки.

// ANCHOR: retry
/// Ошибки записи в EEPROM по шине I2C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    /// Микросхема ещё пишет предыдущую страницу: можно повторить.
    Busy,
    /// Адрес не подтверждён: повторять бесполезно.
    Nack,
}

/// Повторяет запись, пока шина занята, но не больше `max` попыток.
/// `loop` возвращает значение через `break`: число попыток или ошибку.
pub fn write_with_retry(
    mut try_write: impl FnMut() -> Result<(), WriteError>,
    max: u32,
) -> Result<u32, WriteError> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        match try_write() {
            Ok(()) => break Ok(attempts),
            Err(WriteError::Busy) if attempts < max => continue,
            Err(e) => break Err(e),
        }
    }
}
// ANCHOR_END: retry

// ANCHOR: labels
/// Байт, которым приёмник помечает повреждённый символ.
pub const ERR: u8 = 0xFF;

/// Сумма байтов всех целых кадров. Кадр с байтом `ERR` пропускается
/// целиком: `continue 'frames` переходит к следующему кадру из
/// внутреннего цикла.
pub fn sum_good_frames(frames: &[&[u8]]) -> u32 {
    let mut total = 0;
    'frames: for frame in frames {
        let mut sum = 0;
        for &b in frame.iter() {
            if b == ERR {
                continue 'frames;
            }
            sum += u32::from(b);
        }
        total += sum;
    }
    total
}
// ANCHOR_END: labels

// ANCHOR: for_desugar
/// Обычный `for`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_sum_for(xs: &[u32]) -> u32 {
    let mut s = 0u32;
    for &x in xs {
        s = s.wrapping_add(x);
    }
    s
}

/// То, во что компилятор разворачивает `for`: итератор и `loop` с
/// `match` по `next()`.
#[inline(never)]
#[unsafe(no_mangle)]
#[expect(clippy::while_let_loop, reason = "показана развёртка for")]
pub fn ch02_sum_desugared(xs: &[u32]) -> u32 {
    let mut s = 0u32;
    let mut iter = IntoIterator::into_iter(xs);
    loop {
        match iter.next() {
            Some(&x) => s = s.wrapping_add(x),
            None => break,
        }
    }
    s
}
// ANCHOR_END: for_desugar

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_until_ready() {
        let mut replies = [Err(WriteError::Busy), Err(WriteError::Busy), Ok(())].into_iter();
        assert_eq!(write_with_retry(|| replies.next().unwrap(), 5), Ok(3));
    }

    #[test]
    fn retry_gives_up() {
        let mut calls = 0;
        let r = write_with_retry(
            || {
                calls += 1;
                Err(WriteError::Busy)
            },
            4,
        );
        assert_eq!((r, calls), (Err(WriteError::Busy), 4));
    }

    #[test]
    fn nack_is_final() {
        let mut calls = 0;
        let r = write_with_retry(
            || {
                calls += 1;
                Err(WriteError::Nack)
            },
            4,
        );
        assert_eq!((r, calls), (Err(WriteError::Nack), 1));
    }

    #[test]
    fn labels() {
        let frames: [&[u8]; 3] = [&[1, 2, 3], &[4, ERR, 5], &[10]];
        assert_eq!(sum_good_frames(&frames), 16);
    }

    #[test]
    fn desugared_is_the_same() {
        let xs: Vec<u32> = (0..1000).map(|i| i * 7919).collect();
        assert_eq!(ch02_sum_for(&xs), ch02_sum_desugared(&xs));
        assert_eq!(ch02_sum_for(&[]), 0);
    }
}
