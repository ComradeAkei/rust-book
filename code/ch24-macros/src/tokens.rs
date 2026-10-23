//! 24.1: `token_tree!` — дерево токенов, разобранное одним API `proc_macro`,
//! без syn и proc_macro2. Вход — поток токенов, выход — один строковый
//! литерал с описанием каждого токена.

use proc_macro::{Delimiter, Literal, Spacing, TokenStream, TokenTree};
use std::fmt::Write;

// ANCHOR: token_tree
pub fn token_tree(input: TokenStream) -> TokenStream {
    if input.is_empty() {
        // Паника процедурного макроса — тоже ошибка компиляции, но без
        // хорошего спана и с пометкой «proc macro panicked» (раздел 24.3).
        panic!("token_tree!: пустой вход, показывать нечего");
    }
    let mut out = String::new();
    dump(input, 0, &mut out);
    // Результат макроса — тоже поток токенов: здесь из одного литерала.
    TokenTree::Literal(Literal::string(&out)).into()
}

fn dump(stream: TokenStream, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    for tt in stream {
        match tt {
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    // Невидимые скобки: так приходит подставленный
                    // фрагмент macro_rules!, например $e:expr.
                    Delimiter::None => ("«", "»"),
                };
                writeln!(out, "{pad}Group {open}").unwrap();
                dump(g.stream(), depth + 1, out);
                writeln!(out, "{pad}{close}").unwrap();
            }
            TokenTree::Ident(i) => writeln!(out, "{pad}Ident {i}").unwrap(),
            TokenTree::Punct(p) => {
                let spacing = match p.spacing() {
                    Spacing::Joint => "Joint",
                    Spacing::Alone => "Alone",
                };
                writeln!(out, "{pad}Punct {} {spacing}", p.as_char()).unwrap();
            }
            TokenTree::Literal(l) => writeln!(out, "{pad}Literal {l}").unwrap(),
        }
    }
}
// ANCHOR_END: token_tree
