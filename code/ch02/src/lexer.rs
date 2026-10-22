//! Задача 2.C: токены мини-языка команд станции телеметрии.
//!
//! Лексеры пишет студент (`exercises/ch02/src/c_lexer.rs`), а типы токенов
//! и генератор тестовых программ даны здесь, чтобы заготовки, решения и
//! замер скорости пользовались одними и теми же.
//!
//! Язык:
//!
//! - пробелы: ` `, `\t`, `\r`, `\n`;
//! - комментарии: `//` до конца строки и `/* … */` без вложенности;
//! - идентификаторы: `[A-Za-z_][A-Za-z0-9_]*`;
//! - целые: `[0-9][0-9_]*`, `0x[0-9a-fA-F_]+`, `0b[01_]+`, в каждом хотя
//!   бы одна цифра после префикса, значение помещается в `u64`; сразу за
//!   числом не может стоять буква, цифра или `_`;
//! - строки: `"…"` без перевода строки внутри, экранирование `\\`, `\"`,
//!   `\n`, `\t`, `\0`, `\xHH`;
//! - операторы: `->` `==` `!=` `<=` `>=` `<<` `>>` `&&` `||` и
//!   `+ - * / % = < > & | ! ( ) { } [ ] , ; . :`, всегда самый длинный.

/// Оператор или знак препинания.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Arrow,
    EqEq,
    Ne,
    Le,
    Ge,
    Shl,
    Shr,
    AndAnd,
    OrOr,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    Lt,
    Gt,
    Amp,
    Pipe,
    Bang,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Dot,
    Colon,
}

impl Op {
    /// Все операторы с их записью: сначала двухсимвольные.
    pub const ALL: [(&'static str, Op); 30] = [
        ("->", Op::Arrow),
        ("==", Op::EqEq),
        ("!=", Op::Ne),
        ("<=", Op::Le),
        (">=", Op::Ge),
        ("<<", Op::Shl),
        (">>", Op::Shr),
        ("&&", Op::AndAnd),
        ("||", Op::OrOr),
        ("+", Op::Plus),
        ("-", Op::Minus),
        ("*", Op::Star),
        ("/", Op::Slash),
        ("%", Op::Percent),
        ("=", Op::Assign),
        ("<", Op::Lt),
        (">", Op::Gt),
        ("&", Op::Amp),
        ("|", Op::Pipe),
        ("!", Op::Bang),
        ("(", Op::LParen),
        (")", Op::RParen),
        ("{", Op::LBrace),
        ("}", Op::RBrace),
        ("[", Op::LBracket),
        ("]", Op::RBracket),
        (",", Op::Comma),
        (";", Op::Semi),
        (".", Op::Dot),
        (":", Op::Colon),
    ];

    /// Запись оператора.
    pub fn text(self) -> &'static str {
        Op::ALL.iter().find(|&&(_, op)| op == self).map(|&(t, _)| t).unwrap()
    }
}

// ANCHOR: types
/// Вид токена. Текст идентификатора и строки не копируется: его можно
/// взять из исходника по `start..end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ident,
    Int(u64),
    Str,
    Op(Op),
}

/// Токен и его место в исходнике, смещения в байтах.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

/// Ошибка лексера и смещение в байтах, к которому она относится.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexError {
    /// Символ, с которого не начинается ни один токен.
    UnexpectedChar(usize),
    /// Строка без закрывающей кавычки: смещение открывающей.
    UnterminatedString(usize),
    /// Комментарий без `*/`: смещение `/*`.
    UnterminatedComment(usize),
    /// Неизвестное экранирование: смещение обратной косой черты.
    BadEscape(usize),
    /// Недопустимый символ в числе или сразу за ним, а также число без
    /// цифр после префикса: смещение этого символа (или конца входа).
    BadDigit(usize),
    /// Значение не помещается в `u64`: смещение начала числа.
    Overflow(usize),
}
// ANCHOR_END: types

/// Генератор случайных корректных программ с заранее известными токенами:
/// для тестов и замера скорости.
#[cfg(not(target_os = "none"))]
pub mod generate {
    use super::{Kind, Op, Token};
    use std::string::String;
    use std::vec::Vec;

    /// Генератор xorshift, чтобы не тянуть крейт rand.
    pub struct Rng(pub u64);

    impl Rng {
        pub fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        pub fn below(&mut self, n: u64) -> u64 {
            self.next_u64() % n
        }

        fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
            &xs[self.below(xs.len() as u64) as usize]
        }
    }

    const IDENT_START: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_";
    const IDENT_REST: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789";
    const SEPARATORS: &[&str] = &[
        " ",
        " ",
        " ",
        "\n",
        "\t",
        "  ",
        "\r\n",
        "// комментарий: */ \"не строка\"\n",
        "/* блок\n * со звёздочками ** и / */",
        "/**/",
    ];
    const STR_PARTS: &[&str] = &[
        "a", "Z", "0", " ", "_", "/*", "//", "'", "ё", "°C", "🛰", "\\\\", "\\\"", "\\n", "\\t",
        "\\0", "\\x41", "\\x7f", "\\xFF",
    ];

    fn ident(rng: &mut Rng, s: &mut String) {
        s.push(char::from(*rng.pick(IDENT_START)));
        for _ in 0..rng.below(9) {
            s.push(char::from(*rng.pick(IDENT_REST)));
        }
    }

    /// Число в случайной системе счисления с разделителями `_`.
    fn int(rng: &mut Rng, s: &mut String) -> u64 {
        let value = match rng.below(4) {
            0 => rng.below(10),
            1 => rng.below(100_000),
            2 => rng.next_u64(),
            _ => u64::MAX - rng.below(3),
        };
        let (prefix, radix) = *rng.pick(&[("", 10), ("0x", 16), ("0b", 2)]);
        let digits = match radix {
            10 => std::format!("{value}"),
            16 => std::format!("{value:x}"),
            _ => std::format!("{value:b}"),
        };
        s.push_str(prefix);
        if !prefix.is_empty() && rng.below(4) == 0 {
            s.push('_');
        }
        for (i, d) in digits.chars().enumerate() {
            if i > 0 && rng.below(6) == 0 {
                s.push('_');
            }
            let d = if radix == 16 && rng.below(2) == 0 { d.to_ascii_uppercase() } else { d };
            s.push(d);
        }
        if rng.below(8) == 0 {
            s.push('_');
        }
        value
    }

    fn string(rng: &mut Rng, s: &mut String) {
        s.push('"');
        for _ in 0..rng.below(8) {
            s.push_str(rng.pick(STR_PARTS));
        }
        s.push('"');
    }

    /// Можно ли поставить `next` вплотную за `prev`, не изменив разбор.
    fn can_touch(prev: Kind, prev_text: &str, next: Kind, next_text: &str) -> bool {
        let word = |k: Kind| matches!(k, Kind::Ident | Kind::Int(_));
        match (prev, next) {
            (Kind::Str, _) | (_, Kind::Str) => true,
            (p, n) if word(p) && word(n) => false,
            (Kind::Op(_), Kind::Op(_)) => {
                let pair = [prev_text.as_bytes()[prev_text.len() - 1], next_text.as_bytes()[0]];
                !matches!(
                    &pair,
                    b"->"
                        | b"=="
                        | b"!="
                        | b"<="
                        | b">="
                        | b"<<"
                        | b">>"
                        | b"&&"
                        | b"||"
                        | b"//"
                        | b"/*"
                )
            }
            _ => true,
        }
    }

    /// Программа примерно из `tokens` токенов и её правильный разбор.
    pub fn program(seed: u64, tokens: usize) -> (String, Vec<Token>) {
        let mut rng = Rng(seed | 1);
        let mut src = String::new();
        let mut expected: Vec<Token> = Vec::with_capacity(tokens);
        let mut prev: Option<(Kind, usize)> = None;
        for _ in 0..tokens {
            let mut text = String::new();
            let kind = match rng.below(10) {
                0..=2 => {
                    ident(&mut rng, &mut text);
                    Kind::Ident
                }
                3 | 4 => Kind::Int(int(&mut rng, &mut text)),
                5 => {
                    string(&mut rng, &mut text);
                    Kind::Str
                }
                _ => {
                    let &(t, op) = rng.pick(&Op::ALL);
                    text.push_str(t);
                    Kind::Op(op)
                }
            };
            if let Some((pk, pstart)) = prev {
                let prev_text = &src[pstart..];
                let touch = can_touch(pk, prev_text, kind, &text) && rng.below(3) == 0;
                if !touch {
                    let mut sep = *rng.pick(SEPARATORS);
                    // Комментарий сразу за `/` превратился бы в `//`.
                    while pk == Kind::Op(Op::Slash) && sep.starts_with('/') {
                        sep = rng.pick(SEPARATORS);
                    }
                    src.push_str(sep);
                }
            }
            let start = src.len();
            src.push_str(&text);
            expected.push(Token { kind, start, end: src.len() });
            prev = Some((kind, start));
        }
        if rng.below(2) == 0 {
            src.push_str(" // хвост без перевода строки");
        }
        (src, expected)
    }
}
