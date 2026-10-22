//! Раздел 1.7: приведения `as`, `From` и `TryFrom`.
//!
//! ```text
//! cargo asm -p ch01 --lib ch01_f32_to_i32
//! cargo asm -p ch01 --lib --target thumbv7em-none-eabihf ch01_f32_to_i32
//! ```

// ANCHOR: as_int
/// Целое в целое: усечение до младших битов, а при расширении — знаковое
/// или нулевое продолжение по типу источника.
pub fn as_examples() -> (u8, i8, u32, i32) {
    (
        0x1234_u16 as u8,       // 0x34: остались младшие 8 бит
        200_u8 as i8,           // -56: те же биты, другое толкование
        -2_i8 as u32,           // 0xFFFF_FFFE: знаковое расширение источника
        0xFFFF_FFFF_u32 as i32, // -1
    )
}
// ANCHOR_END: as_int

// ANCHOR: as_float
/// Число с плавающей точкой в целое: отбрасывание дробной части к нулю,
/// насыщение на границах, NaN превращается в 0. Неопределённого поведения
/// нет ни на каком входе.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_f32_to_i32(x: f32) -> i32 {
    x as i32
}

/// Для сравнения: явный выбор поведения через проверку диапазона.
pub fn f32_to_i32_checked(x: f32) -> Option<i32> {
    // 2³¹ точно представимо в f32, а i32::MAX — нет: сравнивать надо с 2³¹.
    // NaN не попадает ни в какой диапазон.
    if (-2_147_483_648.0..2_147_483_648.0).contains(&x) { Some(x as i32) } else { None }
}
// ANCHOR_END: as_float

// ANCHOR: from
/// `From` есть только там, где преобразование не теряет данных ни на одной
/// платформе. `usize::from(u32)` не существует: на 16-битной цели оно
/// теряло бы биты.
pub fn widen(x: u16) -> (u32, usize, f64) {
    (u32::from(x), usize::from(x), f64::from(x))
}

/// `TryFrom` проверяет диапазон и возвращает ошибку вместо усечения.
pub fn narrow(x: i64) -> Result<u8, core::num::TryFromIntError> {
    u8::try_from(x)
}
// ANCHOR_END: from

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_casts() {
        assert_eq!(as_examples(), (0x34, -56, 0xFFFF_FFFE, -1));
    }

    #[test]
    fn float_casts_saturate() {
        assert_eq!(ch01_f32_to_i32(2.9), 2);
        assert_eq!(ch01_f32_to_i32(-2.9), -2);
        assert_eq!(ch01_f32_to_i32(1e10), i32::MAX);
        assert_eq!(ch01_f32_to_i32(-1e10), i32::MIN);
        assert_eq!(ch01_f32_to_i32(f32::NAN), 0);
        assert_eq!(ch01_f32_to_i32(f32::INFINITY), i32::MAX);
        assert_eq!(f32_to_i32_checked(2_147_483_648.0), None);
        assert_eq!(f32_to_i32_checked(-2_147_483_648.0), Some(i32::MIN));
        assert_eq!(f32_to_i32_checked(f32::NAN), None);
    }

    #[test]
    fn int_to_float_rounds() {
        // Выше 2²⁴ представимы только чётные целые, ничья — к чётной мантиссе.
        assert_eq!(16_777_219_i32 as f32, 16_777_220.0);
        assert_eq!(16_777_218_i32 as f32, 16_777_218.0);
        assert_eq!(i32::MAX as f32, 2_147_483_648.0);
        assert_eq!(u128::MAX as f32, f32::INFINITY);
    }

    #[test]
    fn from_and_try_from() {
        assert_eq!(widen(65535), (65535, 65535, 65535.0));
        assert_eq!(narrow(255), Ok(255));
        assert!(narrow(256).is_err());
        assert!(narrow(-1).is_err());
    }
}
