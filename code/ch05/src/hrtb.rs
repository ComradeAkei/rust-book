//! Раздел 5.5: ограничения для всех времён жизни, `for<'a>`.
//!
//! Если время жизни аргумента замыкания сделать параметром функции, оно
//! обязано длиться весь вызов `total_named`, и ссылка на локальный буфер
//! не подходит:
//!
//! ```compile_fail,E0597
//! use ch05::hrtb::unescape;
//!
//! pub fn total_named<'x, F>(text: &'x str, mut f: F) -> u32
//! where
//!     F: FnMut(&'x str) -> Option<u32>,
//! {
//!     let mut buf = String::new();
//!     let mut sum = 0;
//!     for line in text.lines() {
//!         let line = if line.contains('\\') {
//!             unescape(line, &mut buf);
//!             &buf[..]
//!         } else {
//!             line
//!         };
//!         sum += f(line).unwrap_or(0);
//!     }
//!     sum
//! }
//! ```
//!
//! Замыкание, которое возвращает часть аргумента, без подсказки не
//! компилируется: компилятор выводит его сигнатуру с одним временем жизни
//! на входе и другим на выходе.
//!
//! ```compile_fail
//! let skip_header = |x: &[u8]| &x[1..];
//! assert_eq!(skip_header(&[0x7E, 1, 2]), [1, 2]);
//! ```
//!
//! Аннотация возвращаемого типа не помогает:
//!
//! ```compile_fail
//! let skip_header = |x: &[u8]| -> &[u8] { &x[1..] };
//! assert_eq!(skip_header(&[0x7E, 1, 2]), [1, 2]);
//! ```

use alloc::string::String;

/// Убирает обратную косую черту перед символом: `r\x=5` → `rx=5`.
pub fn unescape(line: &str, buf: &mut String) {
    buf.clear();
    let mut esc = false;
    for c in line.chars() {
        if esc || c != '\\' {
            buf.push(c);
            esc = false;
        } else {
            esc = true;
        }
    }
}

// ANCHOR: total
/// Сумма значений по строкам конфигурации. Строку с экранированием функция
/// сначала переписывает в локальный буфер, и `f` получает ссылку на него.
/// Такое время жизни не назвать параметром `total_after`: буфер моложе
/// вызова. `for<'a>` требует, чтобы `f` принимала ссылку любой длины жизни.
pub fn total_after<F>(text: &str, mut f: F) -> u32
where
    F: for<'a> FnMut(&'a str) -> Option<u32>,
{
    let mut buf = String::new();
    let mut sum = 0;
    for line in text.lines() {
        let line = if line.contains('\\') {
            unescape(line, &mut buf);
            &buf[..]
        } else {
            line
        };
        sum += f(line).unwrap_or(0);
    }
    sum
}

/// То же без явного `for<'a>`: `FnMut(&str)` с пропущенным временем
/// жизни — сокращённая запись той же границы.
pub fn total_after_short<F: FnMut(&str) -> Option<u32>>(text: &str, f: F) -> u32 {
    total_after(text, f)
}
// ANCHOR_END: total

// ANCHOR: sum_all
/// Сумма по любой коллекции, которую можно обойти по ссылке: `Vec<u32>`,
/// массив, `BTreeSet<u32>`. Граница стоит на типе `&'a C`, а не на
/// замыкании, и без `for<'a>` её не записать: время жизни ссылки
/// появляется только внутри функции.
pub fn sum_all<C>(c: &C) -> u32
where
    for<'a> &'a C: IntoIterator<Item = &'a u32>,
{
    c.into_iter().sum()
}
// ANCHOR_END: sum_all

// ANCHOR: slicer
/// Подсказка для замыкания: функция с ограничением `for<'a>` задаёт его
/// сигнатуру, и вывод типов перестаёт придумывать два разных времени жизни.
pub fn slicer<F>(f: F) -> F
where
    F: for<'a> Fn(&'a [u8]) -> &'a [u8],
{
    f
}
// ANCHOR_END: slicer

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeSet;
    use alloc::vec;

    fn rx(l: &str) -> Option<u32> {
        l.strip_prefix("rx=")?.parse().ok()
    }

    #[test]
    fn hrtb() {
        let text = "rx=10\nr\\x=5\ntx=7\nrx=2";
        assert_eq!(total_after(text, rx), 17);
        assert_eq!(total_after_short(text, |l| l.strip_prefix("tx=")?.parse().ok()), 7);
        let mut seen = 0;
        total_after(text, |_| {
            seen += 1;
            None
        });
        assert_eq!(seen, 4);
    }

    #[test]
    fn sum_any_collection() {
        assert_eq!(sum_all(&vec![1, 2, 3]), 6);
        assert_eq!(sum_all(&[10, 20]), 30);
        assert_eq!(sum_all(&BTreeSet::from([5, 5, 7])), 12);
    }

    #[test]
    fn closure_signature() {
        let skip_header = slicer(|x| &x[1..]);
        let frame = [0x7E, 1, 2];
        assert_eq!(skip_header(&frame), [1, 2]);
        // Замыкание без захвата приводится к указателю на функцию, тип
        // которого с пропущенным временем жизни — тоже for<'a>.
        let f: fn(&[u8]) -> &[u8] = |x| &x[1..];
        let g: for<'a> fn(&'a [u8]) -> &'a [u8] = f;
        assert_eq!(core::any::type_name_of_val(&g), "fn(&'_ [u8]) -> &'_ [u8]");
        assert_eq!(g(&frame), [1, 2]);
    }
}
