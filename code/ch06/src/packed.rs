//! Раздел 6.7: `repr(packed)` и невыровненный доступ.

// ANCHOR: wire
/// Кадр в том виде, в каком он лежит в проводе: без выравнивания.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct WireSample {
    pub channel: u8,
    pub value: u32,
    pub flags: u8,
}
// ANCHOR_END: wire

// ANCHOR: copy_out
/// Чтение поля по значению разрешено: компилятор сам сгенерирует
/// невыровненную загрузку, подходящую для целевого процессора.
pub fn value_of(s: &WireSample) -> u32 {
    s.value
}

/// Скобки `{ }` создают копию поля, и ссылка берётся уже на неё.
pub fn value_ref_workaround(s: &WireSample) -> u32 {
    let v = { s.value };
    let r: &u32 = &v;
    *r
}
// ANCHOR_END: copy_out

// ANCHOR: raw_ptr
/// Если нужен именно адрес поля, берём сырой указатель через `&raw const`
/// и читаем через `read_unaligned`.
pub fn value_via_ptr(s: &WireSample) -> u32 {
    let p: *const u32 = &raw const s.value;
    // SAFETY: указатель получен из живой ссылки, read_unaligned не требует
    // выравнивания.
    unsafe { p.read_unaligned() }
}
// ANCHOR_END: raw_ptr

// ANCHOR: from_bytes
/// Разбор кадра из байтов без unsafe: порядок байт задаём явно.
pub fn parse(bytes: &[u8; 6]) -> WireSample {
    WireSample {
        channel: bytes[0],
        value: u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
        flags: bytes[5],
    }
}
// ANCHOR_END: from_bytes

/// Ссылка на поле packed-структуры не компилируется (проверяется doc-тестом):
///
/// ```compile_fail,E0793
/// use ch06::packed::WireSample;
/// fn broken(s: &WireSample) -> &u32 {
///     &s.value
/// }
/// ```
pub const E0793_IS_CHECKED_BY_DOCTEST: () = ();

/* ANCHOR: e0793
fn broken(s: &WireSample) -> &u32 {
    &s.value // error[E0793]: reference to field of packed struct is unaligned
}
ANCHOR_END: e0793 */

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn packed_has_no_padding() {
        assert_eq!(size_of::<WireSample>(), 6);
        assert_eq!(align_of::<WireSample>(), 1);
    }

    #[test]
    fn all_reads_agree() {
        let s = parse(&[7, 0x78, 0x56, 0x34, 0x12, 0x80]);
        assert_eq!(value_of(&s), 0x1234_5678);
        assert_eq!(value_ref_workaround(&s), 0x1234_5678);
        assert_eq!(value_via_ptr(&s), 0x1234_5678);
    }
}
