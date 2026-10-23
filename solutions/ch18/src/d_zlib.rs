//! Задача D: обернуть настоящую библиотеку zlib в безопасный API. Сырые
//! объявления — то, что дал бы `bindgen` по `zlib.h`; поверх них — безопасные
//! `compress`/`decompress`, которые сами считают размеры буферов, передают
//! out-параметр длины и переводят код возврата в `Result`. `#[link(name =
//! "z")]` подключает libz.

use core::ffi::{c_int, c_ulong};

// ANCHOR: bind
#[link(name = "z")]
unsafe extern "C" {
    fn compressBound(source_len: c_ulong) -> c_ulong;
    fn compress2(
        dest: *mut u8,
        dest_len: *mut c_ulong,
        source: *const u8,
        source_len: c_ulong,
        level: c_int,
    ) -> c_int;
    fn uncompress(
        dest: *mut u8,
        dest_len: *mut c_ulong,
        source: *const u8,
        source_len: c_ulong,
    ) -> c_int;
}
// ANCHOR_END: bind

// ANCHOR: safe
/// Сжать данные (уровень 0..=9). Буфер назначения — наш `Vec`: `compressBound`
/// даёт достаточную ёмкость, out-параметр `dest_len` возвращает фактическую
/// длину, по которой `Vec` усекается.
pub fn compress(data: &[u8], level: i32) -> Vec<u8> {
    let cap = unsafe { compressBound(data.len() as c_ulong) };
    let mut out = vec![0u8; cap as usize];
    let mut dest_len = cap;
    // SAFETY: out вмещает cap байт (compressBound), source — ровно data.len()
    // байт; dest_len на входе = ёмкость out, на выходе = записанная длина.
    let rc = unsafe {
        compress2(out.as_mut_ptr(), &mut dest_len, data.as_ptr(), data.len() as c_ulong, level)
    };
    assert_eq!(rc, 0, "compress2 вернул код {rc}");
    out.truncate(dest_len as usize);
    out
}

/// Распаковать в буфер известной (ожидаемой) длины. Код возврата zlib,
/// отличный от нуля, превращается в `Err`.
pub fn decompress(data: &[u8], expected_len: usize) -> Result<Vec<u8>, i32> {
    let mut out = vec![0u8; expected_len];
    let mut dest_len = expected_len as c_ulong;
    // SAFETY: out вмещает expected_len байт; source — ровно data.len() байт;
    // dest_len на входе = ёмкость out, на выходе = распакованная длина.
    let rc = unsafe {
        uncompress(out.as_mut_ptr(), &mut dest_len, data.as_ptr(), data.len() as c_ulong)
    };
    if rc != 0 {
        return Err(rc);
    }
    out.truncate(dest_len as usize);
    Ok(out)
}
// ANCHOR_END: safe
