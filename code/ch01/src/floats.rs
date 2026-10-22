//! Раздел 1.4: f32 и f64 по IEEE 754.
//!
//! ```text
//! cargo asm -p ch01 --lib --target thumbv7m-none-eabi ch01_fadd32       # программная
//! cargo asm -p ch01 --lib --target thumbv7em-none-eabihf ch01_fadd32    # FPU
//! cargo asm -p ch01 --lib --target thumbv7em-none-eabihf ch01_fadd64    # снова программная
//! ```

use core::cmp::Ordering;

// ANCHOR: fields
/// Поля числа одинарной точности: знак (1 бит), смещённый порядок (8 бит) и
/// дробная часть мантиссы (23 бита). Ведущая единица мантиссы не хранится.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct F32Fields {
    pub negative: bool,
    pub biased_exp: u8,
    pub fraction: u32,
}

pub fn fields(x: f32) -> F32Fields {
    let bits = x.to_bits();
    F32Fields {
        negative: bits >> 31 != 0,
        biased_exp: (bits >> 23) as u8,
        fraction: bits & 0x7F_FFFF,
    }
}

/// Обратная сборка. Любые 32 бита — допустимое значение f32, поэтому
/// `from_bits` безопасна.
pub fn from_fields(f: F32Fields) -> f32 {
    f32::from_bits(
        (f.negative as u32) << 31 | (f.biased_exp as u32) << 23 | (f.fraction & 0x7F_FFFF),
    )
}
// ANCHOR_END: fields

// ANCHOR: ulp
/// Расстояние до следующего представимого числа (единица последнего
/// разряда, ulp). Растёт вдвое на каждой степени двойки.
pub fn ulp(x: f32) -> f32 {
    x.next_up() - x
}
// ANCHOR_END: ulp

// ANCHOR: order
/// Сортировка с NaN. `sort()` для `[f32]` не компилируется: у f32 нет `Ord`.
/// `total_cmp` задаёт полный порядок IEEE 754 totalOrder:
/// −NaN < −∞ < … < −0 < +0 < … < +∞ < +NaN.
pub fn sort_total(xs: &mut [f32]) {
    xs.sort_unstable_by(f32::total_cmp);
}

/// Наибольшее значение, где NaN считается отсутствием данных.
pub fn max_ignoring_nan(xs: &[f32]) -> Option<f32> {
    xs.iter()
        .copied()
        .filter(|x| !x.is_nan())
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
}
// ANCHOR_END: order

// ANCHOR: soft
/// Сложение f32: `vadd.f32` на Cortex-M4F, вызов `__aeabi_fadd` на M0 и M3.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_fadd32(a: f32, b: f32) -> f32 {
    a + b
}

/// Сложение f64: FPU Cortex-M4F умеет только одинарную точность, поэтому
/// здесь вызов `__aeabi_dadd` на всех трёх ядрах.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_fadd64(a: f64, b: f64) -> f64 {
    a + b
}
// ANCHOR_END: soft

// ANCHOR: no_fma
/// Умножение и сложение двумя операциями с двумя округлениями. Rust не
/// сливает их в одну инструкцию FMA сам: результат изменился бы.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch01_mul_add_separate(a: f32, b: f32, c: f32) -> f32 {
    a * b + c
}
// ANCHOR_END: no_fma

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_of_one() {
        assert_eq!(1.0f32.to_bits(), 0x3F80_0000);
        assert_eq!(fields(1.0), F32Fields { negative: false, biased_exp: 127, fraction: 0 });
        assert_eq!(
            fields(-2.5),
            F32Fields { negative: true, biased_exp: 128, fraction: 0x20_0000 }
        );
        assert_eq!(from_fields(fields(0.1)), 0.1);
        assert_eq!(fields(f32::MIN_POSITIVE).biased_exp, 1);
        assert_eq!(fields(f32::from_bits(1)).biased_exp, 0); // денормализованное
        assert_eq!(
            fields(f32::INFINITY),
            F32Fields { negative: false, biased_exp: 255, fraction: 0 }
        );
        assert!(from_fields(F32Fields { negative: false, biased_exp: 255, fraction: 1 }).is_nan());
    }

    #[test]
    fn ulp_doubles_at_powers_of_two() {
        assert_eq!(ulp(1.0), f32::EPSILON);
        assert_eq!(ulp(2.0), 2.0 * f32::EPSILON);
        assert_eq!(ulp(8_388_608.0), 1.0); // 2²³
        assert_eq!(ulp(16_777_216.0), 2.0); // 2²⁴
    }

    #[test]
    fn nan_and_zero() {
        let nan = f32::NAN;
        assert!(nan != nan);
        assert_eq!(nan.partial_cmp(&1.0), None);
        assert_eq!(core::hint::black_box(0.0f32), -0.0f32);
        assert_ne!(0.0f32.to_bits(), (-0.0f32).to_bits());
        assert_eq!(0.0f32.total_cmp(&-0.0), Ordering::Greater);
        assert_eq!(1.0 / -0.0f32, f32::NEG_INFINITY);
    }

    #[test]
    fn sort_with_nan() {
        let mut v = [3.0, f32::NAN, -0.0, 0.0, f32::NEG_INFINITY, -f32::NAN, 1.0];
        sort_total(&mut v);
        assert!(v[0].is_nan() && v[0].is_sign_negative());
        assert_eq!(&v[1..6], &[f32::NEG_INFINITY, -0.0, 0.0, 1.0, 3.0]);
        assert!(
            v[1..6]
                .iter()
                .zip([f32::NEG_INFINITY, -0.0, 0.0, 1.0, 3.0])
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        assert!(v[6].is_nan() && v[6].is_sign_positive());
        assert_eq!(max_ignoring_nan(&[1.0, f32::NAN, 7.0, 2.0]), Some(7.0));
    }

    #[test]
    fn exact_inputs_agree() {
        // Когда произведение точно представимо, округление одно и то же, и
        // обе формы совпадают. Где они расходятся, выясняет задача 1.D.
        assert_eq!(ch01_mul_add_separate(1.5, 2.0, 0.25), 3.25);
        assert_eq!(1.5f32.mul_add(2.0, 0.25), 3.25);
    }
}
