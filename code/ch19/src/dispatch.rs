//! Раздел 19.4: выбор реализации по фичам процессора во время выполнения.
//! Функция с `#[target_feature(enable = "avx2")]` ([19.3](super::simd)) — UB
//! на процессоре без AVX2, поэтому прежде чем её звать, наличие фичи проверяют
//! `is_x86_feature_detected!`. Проверять на каждый вызов дорого, поэтому
//! выбор делают один раз и кэшируют в указателе на функцию.

use crate::simd;
use std::sync::OnceLock;

// ANCHOR: dispatch
type SumFn = fn(&[i32]) -> i32;

/// Обёртка-мостик: безопасная сигнатура `SumFn` поверх `unsafe` AVX2-функции.
/// В кэш попадает только если `best` увидел AVX2.
fn avx2_wrap(data: &[i32]) -> i32 {
    // SAFETY: эта функция выбирается только при обнаруженном AVX2.
    unsafe { simd::sum_avx2(data) }
}

/// Определить лучшую доступную реализацию. Вызывается один раз.
fn best() -> SumFn {
    if is_x86_feature_detected!("avx2") { avx2_wrap } else { simd::sum_sse2 }
}

static DISPATCH: OnceLock<SumFn> = OnceLock::new();

/// Сумма `i32` лучшей доступной реализацией. Фичи процессора определяются при
/// первом вызове и кэшируются: дальше — проверка `OnceLock` и косвенный вызов
/// по закэшированному указателю, без повторного `cpuid`.
pub fn sum(data: &[i32]) -> i32 {
    DISPATCH.get_or_init(best)(data)
}
// ANCHOR_END: dispatch

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_matches_scalar() {
        let data: Vec<i32> = (0..100).map(|x| x - 50).collect();
        let scalar: i32 = data.iter().sum();
        assert_eq!(sum(&data), scalar);
        // Второй вызов идёт уже через кэш.
        assert_eq!(sum(&data), scalar);
    }
}
