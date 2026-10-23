//! Задача D: оберните настоящую zlib в безопасный API. Условие — раздел 18.7.

// ANCHOR: skeleton
// Объявите из zlib (с `#[link(name = "z")]`):
//   uLong compressBound(uLong sourceLen);
//   int   compress2(Bytef *dest, uLongf *destLen,
//                   const Bytef *source, uLong sourceLen, int level);
//   int   uncompress(Bytef *dest, uLongf *destLen,
//                    const Bytef *source, uLong sourceLen);

/// Сжать данные (уровень 0..=9). Буфер назначения считайте сами через
/// `compressBound`, фактическую длину возьмите из out-параметра.
pub fn compress(data: &[u8], level: i32) -> Vec<u8> {
    let _ = (data, level);
    todo!("раздел 18.7, задача D")
}

/// Распаковать в буфер ожидаемой длины; код возврата != 0 — это `Err`.
pub fn decompress(data: &[u8], expected_len: usize) -> Result<Vec<u8>, i32> {
    let _ = (data, expected_len);
    todo!("раздел 18.7, задача D")
}
// ANCHOR_END: skeleton
