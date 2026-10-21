//! Раздел 6.2: раскладка структур по умолчанию и `repr(C)`.
//!
//! Тесты делятся на две группы. `guaranteed` проверяет то, что обещает
//! Reference, и не сломается ни на одной версии компилятора. `observed`
//! фиксирует поведение текущего rustc: оно может измениться, и именно об
//! этом говорит глава.

// ANCHOR: sample_rust
/// Отсчёт датчика в раскладке по умолчанию (`repr(Rust)`).
pub struct Sample {
    pub channel: u8,
    pub value: u32,
    pub flags: u8,
}
// ANCHOR_END: sample_rust

// ANCHOR: sample_c
/// Тот же отсчёт, но с раскладкой C: поля идут в порядке объявления.
#[repr(C)]
pub struct SampleC {
    pub channel: u8, // смещение 0, затем 3 байта выравнивания
    pub value: u32,  // смещение 4
    pub flags: u8,   // смещение 8, затем 3 байта хвостового выравнивания
}
// ANCHOR_END: sample_c

// ANCHOR: sample_sorted
/// Ручная перестановка полей от больших к меньшим даёт 8 байт и в `repr(C)`.
#[repr(C)]
pub struct SampleSorted {
    pub value: u32,
    pub channel: u8,
    pub flags: u8,
}
// ANCHOR_END: sample_sorted

// ANCHOR: hot
/// Запись журнала, в которой поля объявлены «как удобно читать».
pub struct Record {
    pub kind: u8,
    pub timestamp: u64,
    pub sensor: u16,
    pub quality: u8,
    pub value: u32,
}

/// Та же запись в раскладке C: компилятор обязан сохранить порядок.
#[repr(C)]
pub struct RecordC {
    pub kind: u8,
    pub timestamp: u64,
    pub sensor: u16,
    pub quality: u8,
    pub value: u32,
}
// ANCHOR_END: hot

// ANCHOR: const_assert
// Проверка раскладки при компиляции: если поле добавят или поменяют тип,
// сборка упадёт здесь, а не у получателя кадра на другом конце провода.
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<SampleC>() == 12);
    assert!(align_of::<SampleC>() == 4);
    assert!(offset_of!(SampleC, value) == 4);
    assert!(offset_of!(SampleC, flags) == 8);
    assert!(size_of::<SampleSorted>() == 8);
};
// ANCHOR_END: const_assert

// ANCHOR: offsets_fn
/// Смещения полей `Sample` в текущей сборке. `offset_of!` работает и для
/// `repr(Rust)`, но результат описывает только этот компилятор и эти флаги.
pub const fn sample_offsets() -> [usize; 3] {
    use core::mem::offset_of;
    [offset_of!(Sample, channel), offset_of!(Sample, value), offset_of!(Sample, flags)]
}
// ANCHOR_END: offsets_fn

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    mod guaranteed {
        use super::*;

        #[test]
        fn repr_c_keeps_declaration_order() {
            assert_eq!(offset_of!(SampleC, channel), 0);
            assert_eq!(offset_of!(SampleC, value), 4);
            assert_eq!(offset_of!(SampleC, flags), 8);
            assert_eq!(size_of::<SampleC>(), 12);
        }

        #[test]
        fn size_is_multiple_of_align() {
            assert_eq!(size_of::<Sample>() % align_of::<Sample>(), 0);
            assert_eq!(size_of::<Record>() % align_of::<Record>(), 0);
            assert_eq!(size_of::<RecordC>() % align_of::<RecordC>(), 0);
        }

        #[test]
        fn align_is_max_of_fields() {
            assert_eq!(align_of::<Sample>(), align_of::<u32>());
            assert_eq!(align_of::<Record>(), align_of::<u64>());
        }

        #[test]
        fn rust_layout_is_never_worse_here() {
            // Reference не обещает оптимальную раскладку, но обещает, что
            // repr(Rust) не хуже простого правила «каждое поле выровнено».
            assert!(size_of::<Sample>() <= size_of::<SampleC>());
            assert!(size_of::<Record>() <= size_of::<RecordC>());
        }
    }

    mod observed {
        use super::*;

        #[test]
        fn rustc_reorders_sample() {
            // Наблюдаемое поведение rustc 1.97 на x86_64 и thumbv7m.
            assert_eq!(size_of::<Sample>(), 8);
            assert_eq!(sample_offsets(), [4, 0, 5]);
        }

        #[test]
        fn rustc_reorders_record() {
            assert_eq!(size_of::<Record>(), 16);
            assert_eq!(size_of::<RecordC>(), 24);
            assert_eq!(offset_of!(Record, timestamp), 0);
        }
    }
}
