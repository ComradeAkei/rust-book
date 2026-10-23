//! 24.1: что видит процедурный макрос. `token_tree!` печатает дерево
//! токенов своего входа: группы в скобках, идентификаторы, знаки
//! пунктуации с признаком «слитно со следующим» и литералы.
//!
//! Пустой вход — паника макроса, то есть ошибка компиляции:
//!
//! ```compile_fail
//! const S: &str = ch24::token_tree!();
//! ```

#[cfg(test)]
mod tests {
    use crate::token_tree;

    // ANCHOR: punct
    #[test]
    fn operators_are_puncts() {
        // `+=` и `->` — по два знака: первый слит со следующим (Joint).
        // Выражения нет: `f(1, "a")` — идентификатор и группа в скобках.
        assert_eq!(
            token_tree!(x += f(1, "a") -> 'c'),
            "\
Ident x
Punct + Joint
Punct = Alone
Ident f
Group (
  Literal 1
  Punct , Alone
  Literal \"a\"
)
Punct - Joint
Punct > Alone
Literal 'c'
"
        );
    }
    // ANCHOR_END: punct

    // ANCHOR: literals
    #[test]
    fn literals_and_lifetimes() {
        // Минус — не часть литерала; суффикс — часть; время жизни — апостроф,
        // слитый с идентификатором.
        assert_eq!(
            token_tree!(-1u8 0x1F 2.5e3 b"ok" &'a [T; 4]),
            "\
Punct - Alone
Literal 1u8
Literal 0x1F
Literal 2.5e3
Literal b\"ok\"
Punct & Alone
Punct ' Joint
Ident a
Group [
  Ident T
  Punct ; Alone
  Literal 4
]
"
        );
    }
    // ANCHOR_END: literals

    // ANCHOR: invisible
    /// Фрагмент `$e:expr` из macro_rules! приходит в процедурный макрос
    /// целиком — группой с невидимыми скобками (раздел 23.1: непрозрачные
    /// фрагменты). Фрагменты `ident` и `tt` приходят как есть.
    macro_rules! via_expr {
        ($e:expr) => {
            token_tree!($e * 2)
        };
    }
    macro_rules! via_tt {
        ($($t:tt)*) => {
            token_tree!($($t)* * 2)
        };
    }

    #[test]
    fn expr_fragment_is_invisible_group() {
        assert_eq!(
            via_expr!(1 + 1),
            "\
Group «
  Literal 1
  Punct + Alone
  Literal 1
»
Punct * Alone
Literal 2
"
        );
        assert_eq!(
            via_tt!(1 + 1),
            "\
Literal 1
Punct + Alone
Literal 1
Punct * Alone
Literal 2
"
        );
    }
    // ANCHOR_END: invisible
}
