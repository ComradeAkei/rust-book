//! Задача A: размеры указателей на объекты трейтов и того, на что они
//! указывают.
//!
//! Сначала предскажите ответы без компилятора, затем проверьте через
//! `size_of` и `size_of_val`. Ответы в словах не зависят от разрядности:
//! тест сверяет их и на x86-64, где слово — 8 байт, и при сборке для
//! Cortex-M, где 4.

// ANCHOR: shapes
pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Square(pub u16);

pub struct Rect {
    pub w: u32,
    pub h: u8,
}

impl Shape for Square {
    fn area(&self) -> u32 {
        u32::from(self.0) * u32::from(self.0)
    }
}

impl Shape for Rect {
    fn area(&self) -> u32 {
        self.w * u32::from(self.h)
    }
}
// ANCHOR_END: shapes

// ANCHOR: words
/// Размеры типов в словах `usize`, по порядку:
///
///  0. `&dyn Shape`
///  1. `Box<dyn Shape>`
///  2. `&str`
///  3. `Box<[u8]>`
///  4. `Rc<dyn Shape>`
///  5. `&&dyn Shape`
///  6. `Option<Box<dyn Shape>>`
///  7. `[&dyn Shape; 3]`
///  8. `&[&dyn Shape; 3]`
///  9. `&[&dyn Shape]`
/// 10. `fn(&dyn Shape) -> u32`
/// 11. `Option<(&dyn Shape, u8)>`
pub const fn words() -> [usize; 12] {
    panic!("предскажите без компилятора, затем проверьте через size_of")
}
// ANCHOR_END: words

// ANCHOR: pointees
/// `size_of_val` в байтах для того, на что указывает:
///
/// 0. `&Square(3) as &dyn Shape`
/// 1. `&Rect { w: 3, h: 4 } as &dyn Shape`
/// 2. `"дом"`
/// 3. `Box<[u8]>` из `vec![0u8; 5]`
/// 4. `&[Square(1), Square(2), Square(3)][..]`
/// 5. `Rc<dyn Shape>` из `Rc::new(Rect { w: 1, h: 1 })`: `size_of_val(&*rc)`
pub const fn pointee_bytes() -> [usize; 6] {
    panic!("предскажите без компилятора, затем проверьте через size_of_val")
}
// ANCHOR_END: pointees
