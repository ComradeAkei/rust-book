//! Задача 2.C: лексер как конечный автомат на `loop` и `match`.

use ch02::lexer::{Kind, LexError, Op, Token};

// ANCHOR: states
/// Состояние автомата между двумя байтами входа.
#[derive(Debug, Clone, Copy)]
enum State {
    /// Между токенами.
    Start,
    Ident,
    /// Прочитан `0`: дальше может быть префикс `x` или `b`.
    Zero,
    /// Десятичное число.
    Dec,
    /// После `0x` или `0b`, цифр ещё не было.
    Prefix(u64),
    /// После префикса, хотя бы одна цифра есть.
    Digits(u64),
    Str,
    /// После `\` в строке.
    Escape,
    /// После `\x`: сколько шестнадцатеричных цифр осталось.
    Hex(u8),
    /// Прочитан `/`: оператор или начало комментария.
    Slash,
    LineComment,
    BlockComment,
    /// В блочном комментарии после `*`.
    BlockStar,
    /// Прочитан первый символ, с которого начинается и двухсимвольный
    /// оператор.
    Pair(u8),
}
// ANCHOR_END: states

/// Однобайтовые операторы, которые не бывают началом двухсимвольных.
fn single(c: u8) -> Option<Op> {
    Some(match c {
        b'+' => Op::Plus,
        b'*' => Op::Star,
        b'%' => Op::Percent,
        b'(' => Op::LParen,
        b')' => Op::RParen,
        b'{' => Op::LBrace,
        b'}' => Op::RBrace,
        b'[' => Op::LBracket,
        b']' => Op::RBracket,
        b',' => Op::Comma,
        b';' => Op::Semi,
        b'.' => Op::Dot,
        b':' => Op::Colon,
        _ => return None,
    })
}

/// Оператор из первого символа пары, если второй не подошёл.
fn first_of_pair(c: u8) -> Op {
    match c {
        b'-' => Op::Minus,
        b'=' => Op::Assign,
        b'!' => Op::Bang,
        b'<' => Op::Lt,
        b'>' => Op::Gt,
        b'&' => Op::Amp,
        _ => Op::Pipe,
    }
}

fn digit_value(c: u8) -> Option<u64> {
    match c {
        b'0'..=b'9' => Some(u64::from(c - b'0')),
        b'a'..=b'f' => Some(u64::from(c - b'a' + 10)),
        b'A'..=b'F' => Some(u64::from(c - b'A' + 10)),
        _ => None,
    }
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

// ANCHOR: lex
/// Разбирает исходник целиком или возвращает первую ошибку.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let bytes = src.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4);
    let mut state = State::Start;
    let mut start = 0; // начало текущего токена или комментария
    let mut value = 0u64; // накопленное значение числа
    let mut escape = 0; // где начался `\` в строке
    let mut i = 0;

    'scan: loop {
        let c = bytes.get(i).copied();
        match state {
            State::Start => {
                start = i;
                match c {
                    None => break 'scan Ok(out),
                    Some(b' ' | b'\t' | b'\r' | b'\n') => {}
                    Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => state = State::Ident,
                    Some(b'0') => {
                        value = 0;
                        state = State::Zero;
                    }
                    Some(d @ b'1'..=b'9') => {
                        value = u64::from(d - b'0');
                        state = State::Dec;
                    }
                    Some(b'"') => state = State::Str,
                    Some(b'/') => state = State::Slash,
                    Some(p @ (b'-' | b'=' | b'!' | b'<' | b'>' | b'&' | b'|')) => {
                        state = State::Pair(p)
                    }
                    Some(c) => match single(c) {
                        Some(op) => out.push(Token { kind: Kind::Op(op), start, end: i + 1 }),
                        None => return Err(LexError::UnexpectedChar(i)),
                    },
                }
            }
            State::Ident => {
                if !c.is_some_and(is_word) {
                    out.push(Token { kind: Kind::Ident, start, end: i });
                    state = State::Start;
                    continue 'scan; // этот байт разберёт Start
                }
            }
            State::Zero => match c {
                Some(b'x') => state = State::Prefix(16),
                Some(b'b') => state = State::Prefix(2),
                _ => {
                    state = State::Dec;
                    continue 'scan;
                }
            },
            State::Dec | State::Prefix(_) | State::Digits(_) => {
                let radix = match state {
                    State::Prefix(r) | State::Digits(r) => r,
                    _ => 10,
                };
                match c {
                    Some(b'_') => {}
                    Some(d) if digit_value(d).is_some_and(|v| v < radix) => {
                        value = value
                            .checked_mul(radix)
                            .and_then(|v| v.checked_add(digit_value(d).unwrap()))
                            .ok_or(LexError::Overflow(start))?;
                        if let State::Prefix(r) = state {
                            state = State::Digits(r);
                        }
                    }
                    Some(d) if is_word(d) => return Err(LexError::BadDigit(i)),
                    _ if matches!(state, State::Prefix(_)) => return Err(LexError::BadDigit(i)),
                    _ => {
                        out.push(Token { kind: Kind::Int(value), start, end: i });
                        state = State::Start;
                        continue 'scan;
                    }
                }
            }
            State::Str => match c {
                Some(b'"') => {
                    out.push(Token { kind: Kind::Str, start, end: i + 1 });
                    state = State::Start;
                }
                Some(b'\\') => {
                    escape = i;
                    state = State::Escape;
                }
                None | Some(b'\n') => return Err(LexError::UnterminatedString(start)),
                Some(_) => {}
            },
            State::Escape => match c {
                Some(b'\\' | b'"' | b'n' | b't' | b'0') => state = State::Str,
                Some(b'x') => state = State::Hex(2),
                None => return Err(LexError::UnterminatedString(start)),
                Some(_) => return Err(LexError::BadEscape(escape)),
            },
            State::Hex(left) => match c {
                Some(h) if h.is_ascii_hexdigit() => {
                    state = if left == 2 { State::Hex(1) } else { State::Str };
                }
                None => return Err(LexError::UnterminatedString(start)),
                Some(_) => return Err(LexError::BadEscape(escape)),
            },
            State::Slash => match c {
                Some(b'/') => state = State::LineComment,
                Some(b'*') => state = State::BlockComment,
                _ => {
                    out.push(Token { kind: Kind::Op(Op::Slash), start, end: i });
                    state = State::Start;
                    continue 'scan;
                }
            },
            State::LineComment => match c {
                Some(b'\n') => state = State::Start,
                None => {
                    state = State::Start;
                    continue 'scan;
                }
                Some(_) => {}
            },
            State::BlockComment | State::BlockStar => match (state, c) {
                (_, None) => return Err(LexError::UnterminatedComment(start)),
                (State::BlockStar, Some(b'/')) => state = State::Start,
                (_, Some(b'*')) => state = State::BlockStar,
                _ => state = State::BlockComment,
            },
            State::Pair(first) => {
                let op = match (first, c) {
                    (b'-', Some(b'>')) => Op::Arrow,
                    (b'=', Some(b'=')) => Op::EqEq,
                    (b'!', Some(b'=')) => Op::Ne,
                    (b'<', Some(b'=')) => Op::Le,
                    (b'>', Some(b'=')) => Op::Ge,
                    (b'<', Some(b'<')) => Op::Shl,
                    (b'>', Some(b'>')) => Op::Shr,
                    (b'&', Some(b'&')) => Op::AndAnd,
                    (b'|', Some(b'|')) => Op::OrOr,
                    _ => {
                        let op = first_of_pair(first);
                        out.push(Token { kind: Kind::Op(op), start, end: i });
                        state = State::Start;
                        continue 'scan;
                    }
                };
                out.push(Token { kind: Kind::Op(op), start, end: i + 1 });
                state = State::Start;
            }
        }
        i += 1;
    }
}
// ANCHOR_END: lex
