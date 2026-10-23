//! Раздел 16.4: provenance — у указателя есть не только адрес, но и право
//! обращаться к определённому участку памяти. Арифметика с адресом это
//! право сохраняет, превращение в число и обратно — теряет. Строгий API
//! (`addr`, `with_addr`, `map_addr`) отделяет адрес от provenance явно, и
//! Miri с `-Zmiri-strict-provenance` ловит нарушения.

// ANCHOR: tagged
/// Указатель на выровненный на 8 байт `T` с тремя свободными младшими
/// битами: туда прячут тег состояния. `map_addr` меняет адрес, но несёт
/// provenance исходного указателя — право обращаться к тому же участку.
pub fn tag<T>(p: *mut T, tag: usize) -> *mut T {
    debug_assert!(align_of::<T>() >= 8 && tag < 8);
    // Меняем только младшие биты адреса, provenance сохраняется.
    p.map_addr(|a| a | tag)
}

/// Снять тег: вернуть выровненный указатель с тем же provenance.
pub fn untag<T>(p: *mut T) -> *mut T {
    p.map_addr(|a| a & !0b111)
}

/// Прочитать тег — это уже просто число, provenance не нужно.
pub fn tag_of<T>(p: *mut T) -> usize {
    p.addr() & 0b111
}
// ANCHOR_END: tagged

// ANCHOR: roundtrip
/// Записать тег, снять его и обратиться к значению по очищенному
/// указателю. Всё право доступа идёт от исходного `p`, число адреса — лишь
/// посредник, поэтому пример проходит и строгий provenance.
pub fn with_tag(value: &mut u64, tag_bits: usize) -> (usize, u64) {
    let p: *mut u64 = value;
    let tagged = tag(p, tag_bits);
    let read_tag = tag_of(tagged);
    let clean = untag(tagged);
    // SAFETY: clean несёт provenance p (менялся только адрес), указывает на
    // живой, выровненный, инициализированный u64.
    let v = unsafe { *clean };
    (read_tag, v)
}
// ANCHOR_END: roundtrip

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_roundtrip() {
        let mut x = 0xABCD_u64;
        // Box<u64> выровнен на 8: три младших бита адреса свободны.
        for t in 0..8 {
            let (read, v) = with_tag(&mut x, t);
            assert_eq!((read, v), (t, 0xABCD));
        }
    }

    #[test]
    fn tag_bits_are_in_low_three() {
        let mut x = 0u64;
        let p: *mut u64 = &mut x;
        assert_eq!(tag_of(p), 0, "выровненный указатель: младшие биты нули");
        let t = tag(p, 0b101);
        assert_eq!(tag_of(t), 0b101);
        assert_eq!(untag(t), p);
    }
}
