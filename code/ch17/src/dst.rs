//! Раздел 17.6: типы динамического размера (DST). Срез `[T]` и `str`, а
//! также `dyn Trait` не имеют размера, известного при компиляции, поэтому
//! живут только за указателем — и указатель этот «толстый»: кроме адреса
//! несёт метаданные (длину или таблицу методов). Свой DST делают, положив
//! последним полем структуры безразмерный хвост.
//!
//! У DST нет размера при компиляции, поэтому `size_of` для него не вызвать:
//!
//! ```compile_fail,E0277
//! let _ = core::mem::size_of::<[u8]>(); // размер [u8] неизвестен
//! ```

use alloc::alloc::{alloc, handle_alloc_error};
use alloc::boxed::Box;
use core::alloc::Layout;
use core::ptr;

// ANCHOR: fat
/// Тонкий указатель на `Sized` — одно слово; на срез или `dyn` — два:
/// адрес плюс метаданные.
pub fn pointer_sizes() -> (usize, usize, usize) {
    (
        size_of::<&u8>(),                   // тонкий: адрес
        size_of::<&[u8]>(),                 // срез: адрес + длина
        size_of::<&dyn core::fmt::Debug>(), // dyn: адрес + vtable
    )
}
// ANCHOR_END: fat

// ANCHOR: packet
/// Пакет с заголовком и хвостом переменной длины в ОДНОМ выделении.
/// Безразмерное последнее поле делает весь тип DST: `Box<Packet>` — толстый
/// указатель, метаданные которого хранят длину хвоста.
#[repr(C)]
pub struct Packet {
    pub id: u32,
    pub data: [u8],
}

impl Packet {
    /// Собрать `Box<Packet>` с `data = payload` в одном выделении кучи.
    pub fn new(id: u32, payload: &[u8]) -> Box<Packet> {
        let n = payload.len();
        let (layout, data_off) = Self::layout(n);
        // SAFETY: layout ненулевого размера — в нём как минимум заголовок u32.
        let thin = unsafe { alloc(layout) };
        if thin.is_null() {
            handle_alloc_error(layout);
        }
        // SAFETY: thin — свежее выделение по layout; пишем id в начало и
        // payload по смещению хвоста, ничего не читая из неинициализированного.
        unsafe {
            (thin as *mut u32).write(id);
            ptr::copy_nonoverlapping(payload.as_ptr(), thin.add(data_off), n);
            // Толстый указатель: метаданные (длина) = длина хвоста n.
            let fat = ptr::slice_from_raw_parts_mut(thin, n) as *mut Packet;
            Box::from_raw(fat)
        }
    }

    /// Раскладка `Packet` с хвостом длины `n` и смещение хвоста. Та же, что
    /// выведет компилятор, поэтому `Box` при сбросе освободит правильно.
    fn layout(n: usize) -> (Layout, usize) {
        let header = Layout::new::<u32>();
        let tail = Layout::array::<u8>(n).unwrap();
        let (layout, off) = header.extend(tail).unwrap();
        (layout.pad_to_align(), off)
    }
}
// ANCHOR_END: packet

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_sizes_on_host() {
        let word = size_of::<usize>();
        assert_eq!(pointer_sizes(), (word, 2 * word, 2 * word));
    }

    #[test]
    fn packet_in_one_allocation() {
        let pkt = Packet::new(0xABCD, &[1, 2, 3, 4]);
        assert_eq!(pkt.id, 0xABCD);
        assert_eq!(&pkt.data, &[1, 2, 3, 4]);
        // Длина хвоста — в метаданных толстого указателя.
        assert_eq!(pkt.data.len(), 4);
    }

    #[test]
    fn packet_empty_tail() {
        let pkt = Packet::new(1, &[]);
        assert_eq!(pkt.id, 1);
        assert!(pkt.data.is_empty());
    }
}
