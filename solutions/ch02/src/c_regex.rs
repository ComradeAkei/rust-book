//! Задача 2.C: тот же лексер на регулярных выражениях.

use ch02::lexer::{Kind, LexError, Op, Token};
use regex::Regex;
use std::sync::LazyLock;

// ANCHOR: regex
/// Одно выражение на все токены. Альтернативы проверяются по порядку,
/// поэтому шестнадцатеричное число стоит раньше десятичного, а «плохие»
/// варианты (незакрытые строка и комментарий) — после правильных.
static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
          (?P<space>[\ \t\r\n]+)
        | (?P<line>//[^\n]*)
        | (?P<block>/\*(?s:.)*?\*/)
        | (?P<open_block>/\*)
        | (?P<ident>[A-Za-z_][A-Za-z0-9_]*)
        | (?P<hex>0x[0-9a-fA-F_]*)
        | (?P<bin>0b[01_]*)
        | (?P<dec>[0-9][0-9_]*)
        | (?P<str>"(?:[^"\\\n]|\\[\\"nt0]|\\x[0-9a-fA-F]{2})*")
        | (?P<op>->|==|!=|<=|>=|<<|>>|&&|\|\||[-+*/%=<>&|!(){}\[\],;.:])
        "#,
    )
    .unwrap()
});

/// Значение числа `text`, которое начинается в `start`: цифры после
/// префикса длиной `prefix` без `_`.
fn int_value(text: &str, prefix: usize, radix: u32, start: usize) -> Result<u64, LexError> {
    let clean: String = text[prefix..].chars().filter(|&c| c != '_').collect();
    if clean.is_empty() {
        return Err(LexError::BadDigit(start + text.len()));
    }
    u64::from_str_radix(&clean, radix).map_err(|_| LexError::Overflow(start))
}

pub fn lex_regex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < src.len() {
        let caps = TOKEN
            .captures_at(src, pos)
            .filter(|c| c.get(0).unwrap().start() == pos)
            .ok_or(LexError::UnexpectedChar(pos))?;
        let end = caps.get(0).unwrap().end();
        let text = &src[pos..end];
        let kind = if caps.name("space").is_some()
            || caps.name("line").is_some()
            || caps.name("block").is_some()
        {
            None
        } else if caps.name("open_block").is_some() {
            return Err(LexError::UnterminatedComment(pos));
        } else if caps.name("ident").is_some() {
            Some(Kind::Ident)
        } else if caps.name("hex").is_some() {
            Some(Kind::Int(int_value(text, 2, 16, pos)?))
        } else if caps.name("bin").is_some() {
            Some(Kind::Int(int_value(text, 2, 2, pos)?))
        } else if caps.name("dec").is_some() {
            Some(Kind::Int(int_value(text, 0, 10, pos)?))
        } else if caps.name("str").is_some() {
            Some(Kind::Str)
        } else {
            let op = Op::ALL.iter().find(|&&(t, _)| t == text).unwrap().1;
            Some(Kind::Op(op))
        };
        // За числом не может сразу стоять буква, цифра или `_`: регулярные
        // выражения крейта regex не умеют заглядывать вперёд, проверяем
        // руками.
        if let Some(Kind::Int(_)) = kind
            && src.as_bytes().get(end).is_some_and(|&c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err(LexError::BadDigit(end));
        }
        if let Some(kind) = kind {
            out.push(Token { kind, start: pos, end });
        }
        pos = end;
    }
    Ok(out)
}
// ANCHOR_END: regex
