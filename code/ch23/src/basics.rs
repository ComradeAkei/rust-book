//! Раздел 23.1: `macro_rules!` — правила, фрагменты, порядок сопоставления.
//!
//! Макрос — функция из токенов в токены, которая работает при компиляции.
//! Правило — образец (matcher) и подстановка (transcriber); правила
//! пробуются сверху вниз, побеждает первое подошедшее.
//!
//! Захваченный фрагмент разбирается парсером языка, и отката нет: если
//! парсер фрагмента `expr` споткнулся, следующее правило уже не
//! пробуется — это ошибка компиляции:
//!
//! ```compile_fail
//! macro_rules! first {
//!     ($e:expr) => { "expr" };
//!     ($i:ident +) => { "ident +" };
//! }
//! let _ = first!(a +); // expected expression, found end of macro arguments
//! ```
//!
//! После `expr` допустимы только `=>`, `,` и `;`: иначе граница фрагмента
//! зависела бы от будущих правил грамматики выражений.
//!
//! ```compile_fail
//! macro_rules! bad {
//!     ($a:expr + $b:expr) => {}; // `$a:expr` is followed by `+`
//! }
//! ```
//!
//! Почему `max_once!` привязывает аргументы через `match`, а не через
//! `let`: временное значение из инициализатора `let` умирает в конце этого
//! `let`, а из выражения `match` — в конце всего оператора, где стоит вызов.
//!
//! ```compile_fail,E0716
//! macro_rules! max_let {
//!     ($a:expr, $b:expr) => {{
//!         let a = $a; // временный Vec умирает здесь
//!         let b = $b;
//!         if a > b { a } else { b }
//!     }};
//! }
//! assert_eq!(*max_let!(&Vec::from([1, 5])[1], &3), 5); // temporary value dropped while borrowed
//! ```

// Макросы раздела вызываются из тестов ниже.
#![cfg_attr(not(test), allow(unused_macros))]

// ANCHOR: max
/// Буквальный перенос `#define MAX(a, b) ((a) > (b) ? (a) : (b))`:
/// аргумент подставлен дважды — и дважды вычисляется.
macro_rules! max_twice {
    ($a:expr, $b:expr) => {
        if $a > $b { $a } else { $b }
    };
}

/// Каждый аргумент вычисляется ровно один раз: `match` связывает значения
/// с именами `a` и `b`, а гигиена (раздел 23.3) не даст им столкнуться с
/// именами вызывающего. `match`, а не `let`, — как в `assert_eq!`:
/// временные значения аргументов живут до конца сравнения.
macro_rules! max_once {
    ($a:expr, $b:expr) => {
        match ($a, $b) {
            (a, b) => {
                if a > b {
                    a
                } else {
                    b
                }
            }
        }
    };
}
// ANCHOR_END: max

// ANCHOR: square
/// `expr` — готовый узел дерева: подставленный, он остаётся одним
/// операндом, как будто взят в скобки.
macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

/// `tt` — просто токены: `1 + 2` подставляется как есть, и приоритет
/// операций решает, как в `#define SQUARE(x) x * x`.
macro_rules! square_tt {
    ($($x:tt)*) => {
        $($x)* * $($x)*
    };
}
// ANCHOR_END: square

// ANCHOR: opaque
macro_rules! is_zero {
    (0) => {
        "ноль"
    };
    ($($t:tt)*) => {
        "не ноль"
    };
}

/// Захваченный как `expr` фрагмент непрозрачен: дальше он передаётся
/// одним узлом и с токеном `0` в образце `is_zero!` уже не совпадёт.
macro_rules! via_expr {
    ($e:expr) => {
        is_zero!($e)
    };
}

/// `tt` остаётся токеном, и образец `(0)` его узнаёт.
macro_rules! via_tt {
    ($t:tt) => {
        is_zero!($t)
    };
}
// ANCHOR_END: opaque

// ANCHOR: fragments
/// Каждый вид фрагмента: слово-метка выбирает правило, `stringify!`
/// показывает, что захвачено.
#[rustfmt::skip]
macro_rules! frag {
    (ident $x:ident) => { stringify!($x) };
    (literal $x:literal) => { stringify!($x) };
    (expr $x:expr) => { stringify!($x) };
    (ty $x:ty) => { stringify!($x) };
    (path $x:path) => { stringify!($x) };
    (pat $x:pat) => { stringify!($x) };
    (block $x:block) => { stringify!($x) };
    (stmt $x:stmt) => { stringify!($x) };
    (item $x:item) => { stringify!($x) };
    (lifetime $x:lifetime) => { stringify!($x) };
    (vis $x:vis fn) => { stringify!($x) };
    (meta $x:meta) => { stringify!($x) };
    (tt $x:tt) => { stringify!($x) };
}
// ANCHOR_END: fragments

#[cfg(test)]
mod tests {
    #[test]
    fn max_twice_evaluates_twice() {
        let mut calls = 0;
        let mut next = || {
            calls += 1;
            calls * 10
        };
        // 10 > 3, и вернулся *второй* вызов: 20.
        assert_eq!(max_twice!(next(), 3), 20);
        assert_eq!(calls, 2);
    }

    #[test]
    fn max_once_evaluates_once() {
        let mut calls = 0;
        let mut next = || {
            calls += 1;
            calls * 10
        };
        assert_eq!(max_once!(next(), 3), 10);
        assert_eq!(calls, 1);
        // Имена `a` и `b` внутри макроса не мешают таким же снаружи.
        let (a, b) = (1, 2);
        assert_eq!(max_once!(b, a), 2);
        // Временные значения аргументов живут до конца оператора.
        assert_eq!(*max_once!(&Vec::from([1, 5])[1], &3), 5);
    }

    #[test]
    fn expr_keeps_precedence_tt_does_not() {
        assert_eq!(square!(1 + 2), 9);
        assert_eq!(square_tt!(1 + 2), 5); // 1 + 2 * 1 + 2
    }

    #[test]
    fn captured_expr_is_opaque() {
        assert_eq!(is_zero!(0), "ноль");
        assert_eq!(via_tt!(0), "ноль");
        assert_eq!(via_expr!(0), "не ноль");
    }

    /// Непрозрачны все виды, кроме `ident`, `lifetime` и `tt`: `literal` и
    /// `ty` тоже не сравнить с буквальным токеном.
    #[test]
    fn which_fragments_are_opaque() {
        macro_rules! via_literal {
            ($l:literal) => {
                is_zero!($l)
            };
        }
        assert_eq!(via_literal!(0), "не ноль");

        macro_rules! is_uart {
            (uart) => {
                "uart"
            };
            ($($t:tt)*) => {
                "другое"
            };
        }
        macro_rules! via_ident {
            ($i:ident) => {
                is_uart!($i)
            };
        }
        macro_rules! via_ty {
            ($t:ty) => {
                is_uart!($t)
            };
        }
        assert_eq!(via_ident!(uart), "uart");
        assert_eq!(via_ty!(uart), "другое");
    }

    #[test]
    fn literal_fragment_accepts_minus() {
        // `-1` — литерал для фрагмента `literal`, хотя в языке это унарный
        // минус перед литералом.
        macro_rules! lit {
            ($x:literal) => {
                $x
            };
        }
        assert_eq!(lit!(-1), -1);
        // Пробел между `-` и `1` в тексте `stringify!` — наблюдение:
        // rustc 1.97 печатает `-1`, nightly — `- 1`.
        assert_eq!(frag!(literal - 1).replace(' ', ""), "-1");
    }

    /// В редакции 2024 `expr` совпадает и с `const { ... }`, и с `_`;
    /// прежнее поведение сохранено во фрагменте `expr_2021`.
    #[test]
    fn expr_fragment_edition_2024() {
        macro_rules! which {
            ($e:expr) => {
                "expr"
            };
            ($($t:tt)*) => {
                "другое"
            };
        }
        macro_rules! which_2021 {
            ($e:expr_2021) => {
                "expr"
            };
            ($($t:tt)*) => {
                "другое"
            };
        }
        assert_eq!(which!(const { 1 }), "expr");
        assert_eq!(which_2021!(const { 1 }), "другое");
        assert_eq!(which!(_), "expr");
        assert_eq!(which_2021!(_), "другое");
    }

    /// Текст `stringify!` — наблюдение: формат не обещан и может меняться
    /// между версиями компилятора (здесь rustc 1.97).
    #[test]
    fn observed_fragment_texts() {
        assert_eq!(frag!(ident uart0), "uart0");
        assert_eq!(frag!(expr 1 + 2 * x), "1 + 2 * x");
        assert_eq!(frag!(ty Vec<u8>), "Vec<u8>");
        assert_eq!(frag!(path::core::mem::size_of::<u32>), "::core::mem::size_of::<u32>");
        assert_eq!(frag!(pat Some(x) | None), "Some(x) | None");
        assert_eq!(frag!(block { 1 }), "{ 1 }");
        assert_eq!(frag!(stmt let x = 1), "let x = 1");
        assert_eq!(frag!(item struct S;), "struct S;");
        assert_eq!(frag!(lifetime 'a), "'a");
        assert_eq!(frag!(vis pub(crate) fn), "pub(crate)");
        // `vis` может быть пустым — но только перед следующим токеном:
        // `frag!(vis)` без `fn` — «unexpected end of macro invocation».
        assert_eq!(frag!(vis fn), "");
        assert_eq!(frag!(meta derive(Debug)), "derive(Debug)");
        assert_eq!(frag!(tt [a b]), "[a b]");
    }
}
