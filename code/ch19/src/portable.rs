//! Раздел 19.5: переносимый SIMD (`core::simd`, nightly). Интринсики
//! `core::arch` (19.3) привязаны к архитектуре: `_mm256_add_epi32` есть
//! только на x86. `core::simd` даёт те же операции над типом `Simd<T, N>`,
//! а компилятор опускает их в SSE2/AVX2/NEON — что есть на цели. Код один,
//! безопасный, без `#[target_feature]` и `unsafe`; платит переносимостью за
//! тонкий контроль над конкретной инструкцией.
//!
//! API ещё нестабилен, поэтому модуль только под `feature = "nightly"`.

use core::simd::{Simd, num::SimdInt};

// ANCHOR: portable
/// Сумма `i32` переносимым SIMD. `Simd<i32, 8>` — восемь лэйнов в одном
/// векторе (как у AVX2 из 19.3), но код тут безопасный и не привязан к
/// архитектуре: компилятор опустит `+=` в AVX2, или в две SSE2-операции, или
/// в NEON — смотря какая цель. `chunks_exact` режет на полные векторы, хвост
/// (меньше восьми) добираем скаляром.
pub fn sum(data: &[i32]) -> i32 {
    let mut acc = Simd::<i32, 8>::splat(0);
    let mut chunks = data.chunks_exact(8);
    for chunk in chunks.by_ref() {
        // Безопасно: ни unsafe, ни проверки фич — всё знает компилятор.
        acc += Simd::<i32, 8>::from_slice(chunk);
    }
    acc.reduce_sum() + chunks.remainder().iter().sum::<i32>()
}
// ANCHOR_END: portable

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_matches_scalar() {
        let data: Vec<i32> = (0..53).map(|x| x - 20).collect();
        let scalar: i32 = data.iter().sum();
        assert_eq!(sum(&data), scalar);
    }
}
