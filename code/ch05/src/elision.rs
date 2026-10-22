//! Раздел 5.1: аннотации времён жизни и правила элизии.
//!
//! Две ссылки на входе, ссылка на выходе — компилятор не знает, из какой
//! она взята:
//!
//! ```compile_fail,E0106
//! pub fn longest(a: &str, b: &str) -> &str {
//!     if a.len() >= b.len() { a } else { b }
//! }
//! ```
//!
//! Ссылка на локальную переменную наружу не выходит:
//!
//! ```compile_fail,E0515
//! pub fn label(n: u32) -> &'static str {
//!     let s = format!("канал {n}");
//!     &s
//! }
//! ```
//!
//! Аннотация связывает результат с обоими аргументами, и результат не
//! переживает более короткий из них:
//!
//! ```compile_fail,E0597
//! use ch05::elision::longest;
//!
//! let port = String::from("uart1");
//! let best;
//! {
//!     let mode = String::from("burst");
//!     best = longest(&port, &mode);
//! }
//! println!("{best}");
//! ```
//!
//! `Parser::rest_short` по правилам элизии заимствует сам парсер, и
//! результат не переживает локальную переменную `p`:
//!
//! ```compile_fail,E0515
//! use ch05::elision::Parser;
//!
//! pub fn tail(src: &str) -> &str {
//!     let p = Parser::new(src);
//!     p.rest_short()
//! }
//! ```

// ANCHOR: longest
/// Более длинная из двух строк. `'a` — общий срок, в течение которого
/// живы обе ссылки-аргумента; результат действителен столько же.
pub fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}
// ANCHOR_END: longest

// ANCHOR: elided
/// Одна ссылка на входе: результат по правилу 2 получает её время жизни.
pub fn trim_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

/// То же, записанное полностью.
pub fn trim_cr_explicit<'a>(line: &'a str) -> &'a str {
    line.strip_suffix('\r').unwrap_or(line)
}

/// Правило 1: у каждой ссылки на входе свой параметр. Ссылок на выходе
/// нет, и связывать нечего.
pub fn starts_with_ci(line: &str, prefix: &str) -> bool {
    line.len() >= prefix.len() && line[..prefix.len()].eq_ignore_ascii_case(prefix)
}
// ANCHOR_END: elided

// ANCHOR: parser
/// Курсор по тексту конфигурации. `'s` — время жизни текста, а не парсера.
pub struct Parser<'s> {
    src: &'s str,
    pos: usize,
}

impl<'s> Parser<'s> {
    pub fn new(src: &'s str) -> Self {
        Parser { src, pos: 0 }
    }

    /// Правило 3: результат получает время жизни `&self`, то есть
    /// заимствует парсер, хотя смотрит в текст.
    pub fn rest_short(&self) -> &str {
        &self.src[self.pos..]
    }

    /// Время жизни текста названо явно: результат переживает парсер.
    pub fn rest(&self) -> &'s str {
        &self.src[self.pos..]
    }

    /// Следующее слово до пробела. `&mut self` заимствуется только на время
    /// вызова, слово живёт столько же, сколько текст.
    pub fn word(&mut self) -> Option<&'s str> {
        let rest = self.rest().trim_start();
        if rest.is_empty() {
            return None;
        }
        let start = self.src.len() - rest.len();
        let len = rest.find(' ').unwrap_or(rest.len());
        self.pos = start + len;
        Some(&self.src[start..start + len])
    }
}

/// Остаток строки после первого слова: парсер — локальная переменная, а
/// результат смотрит в `src` и возвращается из функции.
pub fn tail(src: &str) -> &str {
    let mut p = Parser::new(src);
    p.word();
    p.rest()
}
// ANCHOR_END: parser

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elision() {
        assert_eq!(longest("uart1", "spi"), "uart1");
        assert_eq!(trim_cr("baud=9600\r"), trim_cr_explicit("baud=9600\r"));
        assert!(starts_with_ci("UART1 rx", "uart"));
        let mut p = Parser::new("set baud 9600");
        let a = p.word();
        let b = p.word(); // два слова живы одновременно: ни одно не держит p
        assert_eq!((a, b), (Some("set"), Some("baud")));
        assert_eq!(p.rest_short(), " 9600");
        assert_eq!(tail("set baud 9600"), " baud 9600");
    }
}
