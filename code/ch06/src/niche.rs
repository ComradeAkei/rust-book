//! Раздел 6.4: нишевая оптимизация.
//!
//! Ниша — это битовый шаблон, который тип никогда не принимает: 0 у ссылки и
//! `NonZeroU32`, значения 2..=255 у `bool`, всё выше 0x10FFFF и суррогаты у
//! `char`, неиспользованные значения тега у enum. Компилятор кладёт в нишу
//! тег внешнего enum и не тратит на него лишние байты.

use core::cell::Cell;
use core::num::NonZeroU32;

// ANCHOR: sensor_id
/// Идентификатор датчика. Ноль зарезервирован протоколом как «нет датчика»,
/// поэтому `Option<SensorId>` бесплатен.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct SensorId(NonZeroU32);

impl SensorId {
    pub const fn new(raw: u32) -> Option<Self> {
        match NonZeroU32::new(raw) {
            Some(nz) => Some(SensorId(nz)),
            None => None,
        }
    }

    pub const fn get(self) -> u32 {
        self.0.get()
    }
}
// ANCHOR_END: sensor_id

// ANCHOR: custom_niche
/// Свой тип с нишей: у enum без данных свободны все значения тега, кроме трёх.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    A,
    B,
    C,
}

/// Отсчёт канала: `Option<Reading>` не больше самого `Reading`, потому что
/// тег `None` помещается в свободные значения поля `channel`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reading {
    pub value: u16,
    pub channel: Channel,
}
// ANCHOR_END: custom_niche

// ANCHOR: blockers
/// Обёртки, которые прячут нишу. `Cell` построен на `UnsafeCell`, а значение
/// внутри `UnsafeCell` может меняться через разделяемую ссылку, поэтому
/// компилятор не вправе хранить в нём чужой тег.
pub type HiddenByCell = Option<Cell<bool>>;
// ANCHOR_END: blockers

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{MaybeUninit, size_of};
    use core::ptr::NonNull;

    // ANCHOR: guaranteed
    #[test]
    fn documented_niches() {
        // Эти равенства описаны в документации core::option как гарантия.
        assert_eq!(size_of::<Option<&u8>>(), size_of::<&u8>());
        assert_eq!(size_of::<Option<&mut [u8]>>(), size_of::<&mut [u8]>());
        assert_eq!(size_of::<Option<NonNull<u8>>>(), size_of::<*mut u8>());
        assert_eq!(size_of::<Option<NonZeroU32>>(), 4);
        assert_eq!(size_of::<Option<fn()>>(), size_of::<fn()>());
        assert_eq!(size_of::<Option<SensorId>>(), 4);
    }
    // ANCHOR_END: guaranteed

    #[test]
    fn observed_niches() {
        assert_eq!(size_of::<Option<bool>>(), 1);
        assert_eq!(size_of::<Option<Option<bool>>>(), 1);
        assert_eq!(size_of::<Option<char>>(), 4);
        assert_eq!(size_of::<Option<Reading>>(), size_of::<Reading>());
        assert_eq!(size_of::<Option<core::cmp::Ordering>>(), 1);
    }

    // ANCHOR: values
    #[test]
    fn observed_niche_values() {
        // Какие именно значения займут None, решает компилятор. Ниже то, что
        // выбрал rustc 1.97. Все типы здесь без байтов выравнивания, поэтому
        // transmute в целое законен, и Miri с этим согласен.
        // SAFETY: размеры совпадают, все байты инициализированы.
        unsafe {
            assert_eq!(core::mem::transmute::<Option<bool>, u8>(None), 2);
            assert_eq!(core::mem::transmute::<Option<Option<bool>>, u8>(Some(None)), 2);
            assert_eq!(core::mem::transmute::<Option<Option<bool>>, u8>(None), 255);
            assert_eq!(core::mem::transmute::<Option<char>, u32>(None), 0xFFFF_FFFF);
            assert_eq!(core::mem::transmute::<Option<core::cmp::Ordering>, i8>(None), -2);
            // А вот это гарантировано: None для NonZero и ссылок — ноль.
            assert_eq!(core::mem::transmute::<Option<NonZeroU32>, u32>(None), 0);
            assert_eq!(core::mem::transmute::<Option<&u8>, usize>(None), 0);
        }
    }
    // ANCHOR_END: values

    #[test]
    fn no_niche_here() {
        // У u32 и сырого указателя допустимы все битовые шаблоны.
        assert_eq!(size_of::<Option<u32>>(), 8);
        assert_eq!(size_of::<Option<*const u8>>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<HiddenByCell>(), 2);
        assert_eq!(size_of::<Option<MaybeUninit<bool>>>(), 2);
    }

    #[test]
    fn sensor_id_zero_is_none() {
        assert!(SensorId::new(0).is_none());
        assert_eq!(SensorId::new(42).map(SensorId::get), Some(42));
    }
}
