//! Раздел 19.3: интринсики `core::arch` и `#[target_feature]`. Интринсик —
//! это функция-обёртка над одной SIMD-инструкцией. Часть набора (SSE2)
//! гарантирована на всех x86-64 и доступна без условий; остальное (AVX2)
//! включается `#[target_feature(enable = ...)]` — и такая функция `unsafe`,
//! потому что на процессоре без этой фичи её инструкции — UB. Проверять
//! наличие фичи обязан вызывающий (раздел 19.4).

use core::arch::x86_64::*;

// ANCHOR: sse2
/// Сумма `i32` на SSE2 (базовый набор x86-64 — `target_feature` не нужен).
/// Четыре лэйна в одном 128-битном аккумуляторе, потом горизонтальная
/// свёртка; хвост (меньше четырёх) — скаляром.
pub fn sum_sse2(data: &[i32]) -> i32 {
    // SAFETY: SSE2 есть на любом x86-64; loadu не требует выравнивания, читаем
    // ровно по 4 i32 в границах среза, хвост добираем скаляром.
    unsafe {
        let mut acc = _mm_setzero_si128();
        let mut i = 0;
        while i + 4 <= data.len() {
            let v = _mm_loadu_si128(data.as_ptr().add(i).cast());
            acc = _mm_add_epi32(acc, v);
            i += 4;
        }
        let mut lanes = [0i32; 4];
        _mm_storeu_si128(lanes.as_mut_ptr().cast(), acc);
        let mut total = lanes[0] + lanes[1] + lanes[2] + lanes[3];
        while i < data.len() {
            total += data[i];
            i += 1;
        }
        total
    }
}
// ANCHOR_END: sse2

// ANCHOR: avx2
/// То же на AVX2: восемь `i32` за инструкцию. `#[target_feature(enable =
/// "avx2")]` компилирует тело с AVX2-кодом, поэтому функция `unsafe`: вызвать
/// её на процессоре без AVX2 — неопределённое поведение.
///
/// # Safety
///
/// Вызывать только при обнаруженном AVX2 (`is_x86_feature_detected!("avx2")`),
/// иначе инструкции AVX2 — UB уже на первой из них.
#[target_feature(enable = "avx2")]
pub unsafe fn sum_avx2(data: &[i32]) -> i32 {
    // SAFETY: вызывающий гарантировал AVX2 (иначе UB уже на первой
    // инструкции); loadu читают 8 i32 в границах среза без требования
    // выравнивания.
    unsafe {
        let mut acc = _mm256_setzero_si256();
        let mut i = 0;
        while i + 8 <= data.len() {
            let v = _mm256_loadu_si256(data.as_ptr().add(i).cast());
            acc = _mm256_add_epi32(acc, v);
            i += 8;
        }
        let mut lanes = [0i32; 8];
        _mm256_storeu_si256(lanes.as_mut_ptr().cast(), acc);
        let mut total: i32 = lanes.iter().sum();
        while i < data.len() {
            total += data[i];
            i += 1;
        }
        total
    }
}
// ANCHOR_END: avx2

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse2_matches_scalar() {
        let data: Vec<i32> = (0..37).collect();
        let scalar: i32 = data.iter().sum();
        assert_eq!(sum_sse2(&data), scalar);
    }

    #[test]
    fn avx2_matches_scalar_when_available() {
        if !is_x86_feature_detected!("avx2") {
            return; // на машине без AVX2 проверять нечего
        }
        let data: Vec<i32> = (0..37).collect();
        let scalar: i32 = data.iter().sum();
        // SAFETY: только что проверили, что AVX2 доступен.
        assert_eq!(unsafe { sum_avx2(&data) }, scalar);
    }
}
