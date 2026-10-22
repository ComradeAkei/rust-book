//! Введение к главе 5: записи конфигурации, которые ссылаются в буфер
//! строки.
//!
//! Перенос `config.c` один в один: записи смотрят в `line`, а `line`
//! очищается и читается заново. Компилятор отвергает программу:
//!
//! ```compile_fail,E0502
//! use std::io::BufRead;
//!
//! struct Entry<'a> {
//!     key: &'a str,
//!     val: &'a str,
//! }
//!
//! fn parse(line: &str) -> Option<Entry<'_>> {
//!     let (key, val) = line.trim_end().split_once('=')?;
//!     Some(Entry { key, val })
//! }
//!
//! fn main() {
//!     let mut input = std::io::stdin().lock();
//!     let mut line = String::new();
//!     let mut cfg = Vec::new();
//!     while input.read_line(&mut line).unwrap() > 0 {
//!         if let Some(e) = parse(&line) {
//!             cfg.push(e);
//!         }
//!         line.clear(); // как fgets: следующая строка в тот же буфер
//!     }
//!     for e in &cfg {
//!         println!("{} = {}", e.key, e.val);
//!     }
//! }
//! ```

use alloc::vec::Vec;

// ANCHOR: entry
/// Запись «ключ = значение». Обе строки — срезы текста конфигурации,
/// `'a` — время жизни этого текста.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Entry<'a> {
    pub key: &'a str,
    pub val: &'a str,
}

/// Разбор без копирования: весь текст прочитан заранее и не меняется,
/// пока живы записи.
pub fn parse_all(text: &str) -> Vec<Entry<'_>> {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(key, val)| Entry { key: key.trim(), val: val.trim() })
        .collect()
}
// ANCHOR_END: entry

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn entries_point_into_text() {
        let text = String::from("rate=100\nport=uart1\nmode=burst\n\n");
        let cfg = parse_all(&text);
        let pairs: Vec<_> = cfg.iter().map(|e| (e.key, e.val)).collect();
        assert_eq!(pairs, [("rate", "100"), ("port", "uart1"), ("mode", "burst")]);
        // `port` указывает в тот же буфер, на 9 байт от начала.
        assert_eq!(cfg[1].key.as_ptr(), text[9..].as_ptr());
    }
}
