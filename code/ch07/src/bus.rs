//! Раздел 7.1: модель шины I2C и оператор `?`.

// ANCHOR: bus
/// Ошибка обмена по шине I2C.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusError {
    /// Устройство не подтвердило адрес регистра (нет ACK).
    Nack(u8),
    /// Другой ведущий захватил шину.
    ArbitrationLost,
}

/// Датчик температуры на шине: четыре регистра и, для опытов, номер
/// регистра, на котором обмен сорвётся.
pub struct Bus {
    regs: [u8; 4],
    fail_at: Option<u8>,
}

impl Bus {
    pub const fn new(regs: [u8; 4]) -> Self {
        Bus { regs, fail_at: None }
    }

    pub const fn failing_at(regs: [u8; 4], reg: u8) -> Self {
        Bus { regs, fail_at: Some(reg) }
    }

    pub fn read(&mut self, reg: u8) -> Result<u8, BusError> {
        if self.fail_at == Some(reg) {
            return Err(BusError::Nack(reg));
        }
        self.regs.get(reg as usize).copied().ok_or(BusError::Nack(reg))
    }
}
// ANCHOR_END: bus

// ANCHOR: read_temp
/// Температура в десятых долях градуса: старший байт в регистре 0,
/// младший в регистре 1.
pub fn read_temp(bus: &mut Bus) -> Result<i16, BusError> {
    let hi = bus.read(0x00)?;
    let lo = bus.read(0x01)?;
    Ok(i16::from_be_bytes([hi, lo]))
}
// ANCHOR_END: read_temp

// ANCHOR: desugared
/// То же, что `read_temp`, с `?`, раскрытым вручную так, как его описывает
/// Reference. Здесь `From::from` превращает `BusError` в `BusError`, и
/// clippy предлагает его убрать, но в общем случае типы ошибок разные.
#[allow(clippy::useless_conversion)]
pub fn read_temp_desugared(bus: &mut Bus) -> Result<i16, BusError> {
    let hi = match bus.read(0x00) {
        Ok(v) => v,
        Err(e) => return Err(From::from(e)),
    };
    let lo = match bus.read(0x01) {
        Ok(v) => v,
        Err(e) => return Err(From::from(e)),
    };
    Ok(i16::from_be_bytes([hi, lo]))
}
// ANCHOR_END: desugared

// ANCHOR: c_style
/// Как это написали бы на C: код возврата и выходной параметр.
pub fn read_temp_c(bus: &mut Bus, out: &mut i16) -> i32 {
    let Ok(hi) = bus.read(0x00) else { return -1 };
    let Ok(lo) = bus.read(0x01) else { return -1 };
    *out = i16::from_be_bytes([hi, lo]);
    0
}
// ANCHOR_END: c_style

// ANCHOR: option
/// Поле CRC кадра из главы 6: байты 6 и 7, little-endian. `?` на `Option`
/// возвращает `None`, если кадр короче восьми байт.
pub fn stored_crc(frame: &[u8]) -> Option<u16> {
    let lo = *frame.get(6)?;
    let hi = *frame.get(7)?;
    Some(u16::from_le_bytes([lo, hi]))
}
// ANCHOR_END: option

/// `?` на `Option` в функции, которая возвращает `Result`, не работает:
/// `None` не во что превратить.
///
/// ```compile_fail,E0277
/// fn crc_or_err(frame: &[u8]) -> Result<u16, &'static str> {
///     let lo = *frame.get(6)?;
///     let hi = *frame.get(7)?;
///     Ok(u16::from_le_bytes([lo, hi]))
/// }
/// ```
///
/// Явное преобразование `ok_or` превращает отсутствие в ошибку:
///
/// ```
/// fn crc_or_err(frame: &[u8]) -> Result<u16, &'static str> {
///     let lo = *frame.get(6).ok_or("кадр короче 8 байт")?;
///     let hi = *frame.get(7).ok_or("кадр короче 8 байт")?;
///     Ok(u16::from_le_bytes([lo, hi]))
/// }
/// assert_eq!(crc_or_err(&[0; 4]), Err("кадр короче 8 байт"));
/// ```
pub fn option_in_result() {}

#[cfg(test)]
mod tests {
    use super::*;

    const REGS: [u8; 4] = [0x00, 0xE7, 0x55, 0xAA];

    #[test]
    fn question_mark_matches_manual_desugaring() {
        for fail in [None, Some(0), Some(1), Some(2)] {
            let mk = || match fail {
                None => Bus::new(REGS),
                Some(r) => Bus::failing_at(REGS, r),
            };
            let a = read_temp(&mut mk());
            let b = read_temp_desugared(&mut mk());
            let mut out = 0;
            let code = read_temp_c(&mut mk(), &mut out);
            assert_eq!(a, b);
            match a {
                Ok(t) => assert_eq!((code, out), (0, t)),
                Err(_) => assert_eq!(code, -1),
            }
        }
        assert_eq!(read_temp(&mut Bus::new(REGS)), Ok(231));
        assert_eq!(read_temp(&mut Bus::failing_at(REGS, 1)), Err(BusError::Nack(1)));
    }

    #[test]
    fn option_question_mark() {
        assert_eq!(stored_crc(&[0xA5, 1, 1, 0, 0, 0, 0x34, 0x12]), Some(0x1234));
        assert_eq!(stored_crc(&[0xA5, 1, 1, 0, 0, 0, 0x34]), None);
        assert_eq!(stored_crc(&[]), None);
    }
}
