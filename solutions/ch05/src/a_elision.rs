//! Задача 5.A: полные сигнатуры для фрагментов
//! `code/ch05/fragments/a1.rs` … `a8.rs`.
//!
//! Каждая строка — сигнатура с явными временами жизни, равносильная
//! исходной, или `"E0106"`, если элизия к исходной не применима и она не
//! компилируется.

pub const SIGNATURES: [&str; 8] = [
    // Правило 2: одна ссылка на входе.
    "fn trim_cr<'a>(line: &'a str) -> &'a str",
    // Правило 2: `sep` не ссылка, у входа одно время жизни.
    "fn split_kv<'a>(line: &'a str, sep: char) -> (&'a str, &'a str)",
    // Две ссылки на входе, `self` нет: правила молчат.
    "E0106",
    // Правило 1: у каждой ссылки свой параметр, и у ссылки внутри `Vec`.
    "fn count_lines<'a, 'b, 'c>(text: &'a str, lines: &'b mut Vec<&'c str>) -> usize",
    // `Fn(&str)` — граница for<'b>, а у `dyn` в `Box` по умолчанию 'static,
    // не время жизни аргумента.
    "fn make_filter<'a>(prefix: &'a str) -> Box<dyn for<'b> Fn(&'b str) -> bool + 'static>",
    // Правило 3: результат берёт время жизни `&self`.
    "fn name<'a>(&'a self, i: usize) -> &'a str",
    // Правило 3, хотя ссылок на входе две.
    "fn lookup<'a, 'b>(&'a self, key: &'b str) -> Option<&'a String>",
    // Правило 3: слово заимствует парсер, а не текст `'s`.
    "fn next_token<'a>(&'a mut self) -> Option<&'a str>",
];
