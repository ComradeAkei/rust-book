//! Разделы 0.2, 0.3 и 0.5: что показывает ассемблер debug- и release-сборки.
//!
//! ```text
//! cargo asm -p ch00 --lib --dev ch00_add        # профиль dev
//! cargo asm -p ch00 --lib ch00_add              # профиль release
//! cargo asm -p ch00 --lib --llvm ch00_add       # LLVM IR
//! cargo asm -p ch00 --lib ch00_average_bad      # что оптимизатор сделал из UB
//! ```

// ANCHOR: add
/// Обычное сложение. В профиле dev оно проверяет переполнение и паникует,
/// в release заворачивается по модулю 2³².
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00_add(a: u32, b: u32) -> u32 {
    a + b
}
// ANCHOR_END: add

// ANCHOR: add_wrapping
/// Заворачивание, записанное явно, одинаково в любом профиле.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch00_add_wrapping(a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}
// ANCHOR_END: add_wrapping

// ANCHOR: average_bad
/// Функция `average` из `examples/ub.rs`, вынесенная отдельным символом,
/// чтобы её можно было найти в ассемблере. В примере она встроена в `main`,
/// здесь компилятор видит её для произвольного среза.
///
/// # Safety
///
/// Безопасного вызова не существует: для любого непустого среза функция
/// читает элемент за концом. Функция нужна только для чтения ассемблера.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn ch00_average_bad(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for i in 1..=xs.len() {
        // SAFETY: НАРУШЕНО. При i == xs.len() индекс за границей.
        sum += unsafe { *xs.get_unchecked(i) };
    }
    sum / xs.len() as u32
}
// ANCHOR_END: average_bad

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_in_range() {
        assert_eq!(ch00_add(2, 3), 5);
        assert_eq!(ch00_add_wrapping(u32::MAX, 1), 0);
    }

    // Поведение `+` при переполнении зависит от профиля, и тест проверяет
    // ровно то, что обещано для текущего: паника при включённых проверках,
    // заворачивание без них.
    #[test]
    fn add_overflow_follows_profile() {
        let r = std::panic::catch_unwind(|| ch00_add(core::hint::black_box(u32::MAX), 1));
        if cfg!(debug_assertions) {
            assert!(r.is_err(), "в dev-профиле переполнение должно паниковать");
        } else {
            assert_eq!(r.unwrap(), 0);
        }
    }
}
