//! Раздел 17.2: операции с сырыми указателями — `read` и `write`,
//! `copy_nonoverlapping`, `swap`, невыровненное чтение. Ими двигают
//! значения там, где владение само не выражается: внутри коллекций, при
//! разборе буферов, на границе с C. Каждая здесь заперта за безопасной
//! обёрткой с проверенными предусловиями.

// ANCHOR: copy
/// Скопировать первые `src.len()` байт `src` в начало `dst`.
/// `copy_nonoverlapping` — это `memcpy`: побайтовая копия, требующая,
/// чтобы области не пересекались.
pub fn copy_prefix(dst: &mut [u8], src: &[u8]) {
    assert!(dst.len() >= src.len());
    // SAFETY: обе области в границах своих срезов (длина проверена), u8
    // выровнен всегда, а `&mut` и `&` на разные срезы заёмщик не даст
    // сделать пересекающимися — память заведомо не перекрывается.
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), src.len());
    }
}
// ANCHOR_END: copy

// ANCHOR: move
/// Положить в ячейку новое значение и вернуть старое — одной парой
/// `read`/`write`, без требования `T: Copy`. `read` забирает значение как
/// есть (владение переходит нам), `write` кладёт новое, не роняя того, что
/// лежало раньше. Это переписанный вручную `core::mem::replace`.
pub fn replace_at<T>(slot: &mut T, new: T) -> T {
    let p: *mut T = slot;
    // SAFETY: p из &mut T — выровнен, инициализирован, единственная ссылка.
    // old читаем ровно один раз и возвращаем (двойного владения нет), new
    // кладём write-ом, который старое не роняет: мы его уже забрали.
    unsafe {
        let old = p.read();
        p.write(new);
        old
    }
}
// ANCHOR_END: move

// ANCHOR: swap
/// Поменять местами два значения по изменяемым ссылкам через `ptr::swap`.
pub fn swap<T>(a: &mut T, b: &mut T) {
    let pa: *mut T = a;
    let pb: *mut T = b;
    // SAFETY: pa и pb получены из разных &mut — память не пересекается,
    // оба выровнены и инициализированы.
    unsafe { core::ptr::swap(pa, pb) }
}
// ANCHOR_END: swap

// ANCHOR: unaligned
/// Прочитать `u32` (порядок байт little-endian) из буфера по смещению,
/// которое может не быть кратно 4. `read_unaligned` снимает требование
/// выравнивания — ровно для таких разборов заголовков и пакетов.
pub fn read_u32_le_at(buf: &[u8], off: usize) -> Option<u32> {
    if off.checked_add(4)? > buf.len() {
        return None;
    }
    // SAFETY: проверили, что 4 байта от off в границах среза; provenance
    // идёт от buf. read_unaligned не требует кратности адреса.
    let raw = unsafe { (buf.as_ptr().add(off) as *const u32).read_unaligned() };
    Some(u32::from_le(raw))
}
// ANCHOR_END: unaligned

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_prefix_copies_only_src_len() {
        let mut dst = [0u8; 5];
        copy_prefix(&mut dst, &[1, 2, 3]);
        assert_eq!(dst, [1, 2, 3, 0, 0]);
    }

    #[test]
    fn replace_at_returns_old() {
        let mut s = String::from("old");
        let old = replace_at(&mut s, String::from("new"));
        assert_eq!(old, "old");
        assert_eq!(s, "new");
    }

    #[test]
    fn swap_exchanges() {
        let (mut a, mut b) = (1u32, 2);
        swap(&mut a, &mut b);
        assert_eq!((a, b), (2, 1));
    }

    #[test]
    fn read_u32_le_handles_offset_and_bounds() {
        let buf = [0u8, 0x78, 0x56, 0x34, 0x12];
        assert_eq!(read_u32_le_at(&buf, 1), Some(0x1234_5678));
        assert_eq!(read_u32_le_at(&buf, 2), None); // не хватает 4 байт
    }
}
