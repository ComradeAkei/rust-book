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
//! С временем жизни из параметров ссылка для обхода держит буфер весь
//! вызов, и очистить его нельзя:
//!
//! ```compile_fail,E0506
//! pub fn take_sum_named<'x, C>(buf: &'x mut C) -> u32
//! where
//!     C: Default,
//!     &'x C: IntoIterator<Item = &'x u32>,
//! {
//!     let sum = (&*buf).into_iter().sum();
//!     *buf = C::default();
//!     sum
//! }
//! ```
//!
//! Замыкание не может сохранить строку, которую ему дали: она
//! действительна только на время вызова.
//!
//! ```compile_fail,E0521
//! use ch05::hrtb::total_after;
//!
//! let text = String::from("rx=1\nr\\x=2");
//! let mut saved = Vec::new();
//! total_after(&text, |l| {
//!     saved.push(l);
//!     None
//! });
//! println!("{saved:?}");
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

// ANCHOR: take_sum
/// Сумма отсчётов в буфере и очистка буфера, для любой коллекции, которую
/// можно обойти по ссылке: `Vec<u32>`, массива, `BTreeSet<u32>`. Ссылка
/// `&C` для обхода берётся внутри функции и должна умереть до записи в
/// `*buf`. Назвать её время жизни параметром функции нельзя, и это не
/// замыкание: сокращённой записи нет, нужен `for<'a>`.
pub fn take_sum<C>(buf: &mut C) -> u32
where
    C: Default,
    for<'a> &'a C: IntoIterator<Item = &'a u32>,
{
    let sum = (&*buf).into_iter().sum();
    *buf = C::default();
    sum
}
// ANCHOR_END: take_sum

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
    fn take_sum_any_collection() {
        let mut v = vec![1, 2, 3];
        assert_eq!((take_sum(&mut v), v.len()), (6, 0));
        let mut a = [10, 20];
        assert_eq!((take_sum(&mut a), a), (30, [0, 0]));
        let mut set = BTreeSet::from([5, 5, 7]);
        assert_eq!((take_sum(&mut set), set.len()), (12, 0));
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
