//! Раздел 30.5: что встраивается через границу крейта, а что нет.

// ANCHOR: callers
#[inline(never)]
pub fn call_scale(x: u32) -> u32 {
    ch30_dep::scale(x)
}

#[inline(never)]
pub fn call_sum(v: &[u32]) -> u32 {
    ch30_dep::scaled_sum(v)
}

#[inline(never)]
pub fn call_sum_inline(v: &[u32]) -> u32 {
    ch30_dep::scaled_sum_inline(v)
}

#[inline(never)]
pub fn call_sum_generic(v: &[u32]) -> u32 {
    ch30_dep::scaled_sum_generic(v)
}
// ANCHOR_END: callers

// ANCHOR: cold
#[inline(never)]
fn report(x: u32) -> u32 {
    eprintln!("значение {x} вне диапазона");
    0
}

#[cold]
#[inline(never)]
fn report_cold(x: u32) -> u32 {
    eprintln!("значение {x} вне диапазона");
    0
}

/// Без подсказок. Множители у трёх функций разные: с одинаковыми телами
/// LLVM слил бы функции в одну (раздел 30.5).
#[inline(never)]
pub fn check_plain(x: u32) -> u32 {
    if x > 1000 { report(x) } else { x * 3 }
}

/// Вызов холодной функции делает холодной и ветку.
#[inline(never)]
pub fn check_cold(x: u32) -> u32 {
    if x > 1000 { report_cold(x) } else { x * 5 }
}

/// То же подсказкой в самой ветке.
#[inline(never)]
pub fn check_hint(x: u32) -> u32 {
    if x > 1000 {
        std::hint::cold_path();
        report(x)
    } else {
        x * 7
    }
}
// ANCHOR_END: cold

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_results() {
        let v: Vec<u32> = (0..1000).collect();
        let want = call_sum(&v);
        assert_eq!(want, v.iter().map(|&x| call_scale(x)).fold(0, u32::wrapping_add));
        assert_eq!(call_sum_inline(&v), want);
        assert_eq!(call_sum_generic(&v), want);
    }

    #[test]
    fn checks() {
        assert_eq!((check_plain(10), check_cold(10), check_hint(10)), (30, 50, 70));
        assert_eq!((check_plain(1001), check_cold(1001), check_hint(1001)), (0, 0, 0));
    }
}
