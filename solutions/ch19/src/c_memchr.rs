//! Задача C: поиск первого вхождения байта в срезе на SIMD. SSE2 сравнивает
//! 16 байтов за раз, AVX2 — 32; `movemask` собирает результат сравнения в
//! битовую маску, по которой `trailing_zeros` даёт смещение совпадения.
//! Реализацию выбирает рантайм-диспетчер (раздел 19.4).

use core::arch::x86_64::*;
use std::sync::OnceLock;

// ANCHOR: sse2
/// Поиск байта на SSE2: `_mm_cmpeq_epi8` даёт 0xFF в совпавших байтах,
/// `_mm_movemask_epi8` собирает их старшие биты в 16-битную маску. Хвост
/// (меньше 16 байт) — скаляром.
pub fn index_sse2(haystack: &[u8], needle: u8) -> Option<usize> {
    // SAFETY: SSE2 есть на любом x86-64; loadu не требует выравнивания, читаем
    // ровно по 16 байт в границах среза, хвост добираем скаляром.
    unsafe {
        let target = _mm_set1_epi8(needle as i8);
        let mut i = 0;
        while i + 16 <= haystack.len() {
            let chunk = _mm_loadu_si128(haystack.as_ptr().add(i).cast());
            let eq = _mm_cmpeq_epi8(chunk, target);
            let mask = _mm_movemask_epi8(eq) as u32;
            if mask != 0 {
                return Some(i + mask.trailing_zeros() as usize);
            }
            i += 16;
        }
        while i < haystack.len() {
            if haystack[i] == needle {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}
// ANCHOR_END: sse2

// ANCHOR: avx2
/// То же на AVX2: 32 байта за итерацию, маска 32-битная. Функция `unsafe` из-за
/// `#[target_feature]` — наличие AVX2 обязан проверить вызывающий.
///
/// # Safety
///
/// Вызывать только при обнаруженном AVX2 (`is_x86_feature_detected!("avx2")`).
#[target_feature(enable = "avx2")]
pub unsafe fn index_avx2(haystack: &[u8], needle: u8) -> Option<usize> {
    // SAFETY: вызывающий гарантировал AVX2; loadu читает 32 байта в границах
    // среза без требования выравнивания, хвост — скаляром.
    unsafe {
        let target = _mm256_set1_epi8(needle as i8);
        let mut i = 0;
        while i + 32 <= haystack.len() {
            let chunk = _mm256_loadu_si256(haystack.as_ptr().add(i).cast());
            let eq = _mm256_cmpeq_epi8(chunk, target);
            let mask = _mm256_movemask_epi8(eq) as u32;
            if mask != 0 {
                return Some(i + mask.trailing_zeros() as usize);
            }
            i += 32;
        }
        while i < haystack.len() {
            if haystack[i] == needle {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}
// ANCHOR_END: avx2

// ANCHOR: dispatch
type IndexFn = fn(&[u8], u8) -> Option<usize>;

/// Обёртка-мостик: безопасная сигнатура `IndexFn` поверх `unsafe` AVX2-версии.
fn avx2_wrap(haystack: &[u8], needle: u8) -> Option<usize> {
    // SAFETY: эта обёртка попадает в кэш, только если best() увидел AVX2.
    unsafe { index_avx2(haystack, needle) }
}

fn best() -> IndexFn {
    if is_x86_feature_detected!("avx2") { avx2_wrap } else { index_sse2 }
}

static DISPATCH: OnceLock<IndexFn> = OnceLock::new();

/// Поиск байта лучшей доступной реализацией. Фичи процессора определяются при
/// первом вызове и кэшируются указателем на функцию.
pub fn index_of(haystack: &[u8], needle: u8) -> Option<usize> {
    DISPATCH.get_or_init(best)(haystack, needle)
}
// ANCHOR_END: dispatch
