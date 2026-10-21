//! Раздел 6.6: `union` и чем он отличается от enum.

// ANCHOR: float_bits
/// Классический приём из C: посмотреть на биты float через объединение.
#[repr(C)]
pub union FloatBits {
    pub f: f32,
    pub u: u32,
}

pub fn bits_via_union(x: f32) -> u32 {
    let fb = FloatBits { f: x };
    // SAFETY: у f32 и u32 одинаковый размер, а любой набор битов u32 допустим.
    unsafe { fb.u }
}

/// Безопасный путь, который компилируется в тот же код.
pub fn bits_safe(x: f32) -> u32 {
    x.to_bits()
}
// ANCHOR_END: float_bits

// ANCHOR: tagged_by_hand
/// Ручное размеченное объединение, как в C. Тег и данные согласованы только
/// дисциплиной программиста, и компилятор ничем не помогает.
#[repr(C)]
pub struct ValueC {
    pub tag: u8, // 0 = целое, 1 = вещественное
    pub data: ValueData,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ValueData {
    pub int: i32,
    pub real: f32,
}

impl ValueC {
    pub fn as_f64(&self) -> f64 {
        // SAFETY: читаем то поле, на которое указывает тег. Если кто-то
        // записал tag = 1 и int, получим осмысленный, но неверный ответ.
        unsafe {
            match self.tag {
                0 => self.data.int as f64,
                _ => self.data.real as f64,
            }
        }
    }
}
// ANCHOR_END: tagged_by_hand

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn union_matches_to_bits() {
        for x in [0.0f32, -0.0, 1.0, f32::MIN_POSITIVE, f32::INFINITY, f32::NAN] {
            assert_eq!(bits_via_union(x), bits_safe(x));
        }
        assert_eq!(bits_safe(1.0), 0x3F80_0000);
    }

    #[test]
    fn union_size_is_max_field() {
        assert_eq!(size_of::<FloatBits>(), 4);
        assert_eq!(size_of::<ValueC>(), 8);
        // У union нет ниши: компилятор не знает, какое поле активно.
        assert_eq!(size_of::<Option<FloatBits>>(), 8);
    }

    #[test]
    fn manual_tag() {
        let v = ValueC { tag: 1, data: ValueData { real: 2.5 } };
        assert_eq!(v.as_f64(), 2.5);
    }
}
