//! Раздел 6.1: кортежные структуры и newtype.

// ANCHOR: units
/// Единицы измерения как отдельные типы. В памяти это просто `i32`,
/// но сложить миллиметры с сотыми долями градуса компилятор не даст.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Millimeters(pub i32);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct CentiCelsius(pub i32);

impl core::ops::Add for Millimeters {
    type Output = Millimeters;
    fn add(self, rhs: Millimeters) -> Millimeters {
        Millimeters(self.0 + rhs.0)
    }
}
// ANCHOR_END: units

// ANCHOR: struct_kinds
/// Три вида структур: с именованными полями, кортежная и единичная.
pub struct Point3 {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}
pub struct Rgb(pub u8, pub u8, pub u8);
pub struct Tick;
// ANCHOR_END: struct_kinds

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn newtype_is_free() {
        assert_eq!(size_of::<Millimeters>(), size_of::<i32>());
        assert_eq!(Millimeters(5) + Millimeters(7), Millimeters(12));
    }

    #[test]
    fn struct_kinds_sizes() {
        assert_eq!(size_of::<Point3>(), 6);
        assert_eq!(size_of::<Rgb>(), 3);
        assert_eq!(size_of::<Tick>(), 0);
        let _ = (Point3 { x: 1, y: 2, z: 3 }.z, Rgb(1, 2, 3).2, CentiCelsius(0));
    }
}
