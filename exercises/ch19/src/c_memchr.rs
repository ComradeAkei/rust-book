//! Задача C: поиск первого вхождения байта в срезе на SIMD с выбором
//! реализации в рантайме. Условие — раздел 19.7.

// ANCHOR: skeleton
/// Поиск байта на SSE2: сравнивайте по 16 байт (`_mm_cmpeq_epi8`), собирайте
/// результат в маску (`_mm_movemask_epi8`), смещение — `trailing_zeros`.
/// Хвост меньше 16 байт — скаляром. SSE2 есть на любом x86-64, фичу
/// проверять не нужно.
pub fn index_sse2(haystack: &[u8], needle: u8) -> Option<usize> {
    let _ = (haystack, needle);
    todo!("раздел 19.7, задача C")
}

/// То же на AVX2: по 32 байта за раз. Функция `unsafe` из-за
/// `#[target_feature]` — наличие AVX2 проверяет вызывающий.
///
/// # Safety
///
/// Вызывать только при обнаруженном AVX2 (`is_x86_feature_detected!`).
#[target_feature(enable = "avx2")]
pub unsafe fn index_avx2(haystack: &[u8], needle: u8) -> Option<usize> {
    let _ = (haystack, needle);
    todo!("раздел 19.7, задача C")
}

/// Поиск байта лучшей доступной реализацией. Проверьте AVX2 один раз и
/// закэшируйте выбор в указателе на функцию (раздел 19.4).
pub fn index_of(haystack: &[u8], needle: u8) -> Option<usize> {
    let _ = (haystack, needle);
    todo!("раздел 19.7, задача C")
}
// ANCHOR_END: skeleton
