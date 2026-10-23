//! Раздел 17.4: `transmute` и его замены. `transmute::<A, B>` переосмысляет
//! биты `A` как `B`, если их размеры равны. Это самый острый инструмент в
//! языке: он обходит систему типов целиком, и почти всегда у него есть
//! безопасная замена, которая яснее и ничего не нарушает. `transmute`
//! оставляют на случаи, где замены нет.
//!
//! Между типами разного размера `transmute` не компилируется:
//!
//! ```compile_fail,E0512
//! let _: u64 = unsafe { core::mem::transmute(0u32) }; // разные размеры
//! ```

// ANCHOR: bytes
/// Склеить 4 байта в `u32` и разобрать обратно — безопасно и переносимо
/// между порядками байт. Это замена `transmute::<[u8; 4], u32>`, которая
/// вдобавок не зависит от порядка байт платформы.
pub fn u32_from_le(b: [u8; 4]) -> u32 {
    u32::from_le_bytes(b)
}
pub fn u32_to_be(x: u32) -> [u8; 4] {
    x.to_be_bytes()
}
// ANCHOR_END: bytes

// ANCHOR: bits
/// Переосмыслить биты `f32` как `u32`. `transmute` здесь законен (размеры
/// равны, любой битовый образец — валидный `u32`), но `to_bits` делает
/// ровно то же без `unsafe` и без риска перепутать размеры.
pub fn f32_to_bits(x: f32) -> u32 {
    x.to_bits()
}
// ANCHOR_END: bits

// ANCHOR: offset
/// `offset_of!` даёт смещение поля, не создавая значения, — как `offsetof`
/// в C. Нужно для разбора структур по сырой памяти.
#[repr(C)]
pub struct Header {
    pub magic: u32,
    pub flags: u8,
    pub len: u32,
}

pub fn len_offset() -> usize {
    core::mem::offset_of!(Header, len)
}
// ANCHOR_END: offset

// ANCHOR: zst
/// Тип нулевого размера не занимает места и не выделяет его: `[Zst; N]`
/// любого размера — ноль байт, а итерация по нему существует только в
/// системе типов.
pub struct Zst;

pub fn zst_is_zero_sized() -> bool {
    size_of::<Zst>() == 0 && size_of::<[Zst; 1000]>() == 0
}
// ANCHOR_END: zst

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_roundtrip() {
        assert_eq!(u32_from_le([0x78, 0x56, 0x34, 0x12]), 0x1234_5678);
        assert_eq!(u32_to_be(0x1234_5678), [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn bits_match_transmute() {
        assert_eq!(f32_to_bits(1.0), 0x3f80_0000);
    }

    #[test]
    fn header_len_after_padding() {
        // magic 0..4, flags 4, выравнивание до 8, len 8..12.
        assert_eq!(len_offset(), 8);
    }

    #[test]
    fn zst_takes_no_space() {
        assert!(zst_is_zero_sized());
    }
}
