//! Приложение B «Изменения edition 2024, важные для системного кода».
//!
//! Модули из `shared/` компилируются ещё и в крейте `appendix-b-2021` с
//! edition 2021; тесты обоих крейтов показывают, как один и тот же
//! исходник ведёт себя в двух редакциях. Остальные модули используют
//! синтаксис, которого в 2021 не было. Фрагменты для книги отмечены парами
//! `ANCHOR: имя` / `ANCHOR_END: имя`.

pub mod compile_errors;
pub mod ffi;
pub mod rpit;
pub mod statics;

#[path = "shared/misc.rs"]
pub mod misc;
#[path = "shared/temporaries.rs"]
pub mod temporaries;

#[cfg(test)]
mod tests {
    use super::misc::*;
    use super::temporaries::*;
    use std::sync::Mutex;

    // ANCHOR: tests_2024
    #[test]
    fn if_let_temporary_dies_before_else() {
        let log = Log::default();
        if_let_order(&log);
        assert_eq!(*log.borrow(), ["drop scrutinee", "else"]);
    }

    #[test]
    fn else_branch_can_relock() {
        assert!(relock_in_else(&Mutex::new(None)));
    }

    #[test]
    fn tail_temporary_dies_before_locals() {
        let log = Log::default();
        tail_order(&log);
        assert_eq!(*log.borrow(), ["drop temporary", "drop local"]);
    }
    // ANCHOR_END: tests_2024

    #[test]
    fn boxed_slice_yields_values() {
        let b: Box<[u32]> = vec![7, 8].into_boxed_slice();
        assert_eq!(boxed_slice_item(b), "core::option::Option<u32>");
    }

    #[test]
    fn expr_fragment_matches_more() {
        assert_eq!(classify_const_block(), "expr");
        assert_eq!(classify_underscore(), "expr");
    }
}
