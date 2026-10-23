//! Задача C: комбинаторы парсеров на замыканиях и разбор ими ответов
//! AT-команд модема.
//!
//! Парсер — функция из строки в «значение и остаток». Комбинатор берёт
//! парсеры и возвращает новый: замыкание, которое захватило аргументы по
//! значению. Итоговый парсер — одно большое вложенное замыкание, и LLVM
//! встраивает его целиком.

// ANCHOR: parser
/// Значение и непрочитанный остаток; `None` — вход не подошёл.
pub type PResult<'a, T> = Option<(T, &'a str)>;

/// Короткое имя для «функции из `&str` в `PResult`»: общая реализация
/// делает парсером любое подходящее замыкание.
pub trait Parser<'a, T>: Fn(&'a str) -> PResult<'a, T> {}

impl<'a, T, F: Fn(&'a str) -> PResult<'a, T>> Parser<'a, T> for F {}
// ANCHOR_END: parser

// ANCHOR: primitives
/// Ровно строка `t`.
pub fn tag<'a>(t: &'static str) -> impl Parser<'a, &'a str> {
    move |s: &'a str| {
        let rest = s.strip_prefix(t)?;
        Some((&s[..t.len()], rest))
    }
}

/// Десятичное число без знака, хотя бы одна цифра; переполнение `u32` —
/// не число.
pub fn number<'a>() -> impl Parser<'a, u32> {
    |s: &'a str| {
        let end = s.bytes().position(|b| !b.is_ascii_digit()).unwrap_or(s.len());
        if end == 0 {
            return None;
        }
        let mut n = 0u32;
        for b in s[..end].bytes() {
            n = n.checked_mul(10)?.checked_add(u32::from(b - b'0'))?;
        }
        Some((n, &s[end..]))
    }
}

/// Строка в двойных кавычках, без экранирования, как в ответах модемов.
pub fn quoted<'a>() -> impl Parser<'a, &'a str> {
    |s: &'a str| {
        let body = s.strip_prefix('"')?;
        let end = body.find('"')?;
        Some((&body[..end], &body[end + 1..]))
    }
}
// ANCHOR_END: primitives

// ANCHOR: combinators
pub fn map<'a, A, B>(p: impl Parser<'a, A>, f: impl Fn(A) -> B) -> impl Parser<'a, B> {
    move |s| p(s).map(|(a, rest)| (f(a), rest))
}

/// Как `map`, но `f` может отвергнуть значение.
pub fn and_then<'a, A, B>(p: impl Parser<'a, A>, f: impl Fn(A) -> Option<B>) -> impl Parser<'a, B> {
    move |s| {
        let (a, rest) = p(s)?;
        Some((f(a)?, rest))
    }
}

pub fn pair<'a, A, B>(p: impl Parser<'a, A>, q: impl Parser<'a, B>) -> impl Parser<'a, (A, B)> {
    move |s| {
        let (a, rest) = p(s)?;
        let (b, rest) = q(rest)?;
        Some(((a, b), rest))
    }
}

/// `p`, затем `q`; значение — от `p`.
pub fn left<'a, A, B>(p: impl Parser<'a, A>, q: impl Parser<'a, B>) -> impl Parser<'a, A> {
    map(pair(p, q), |(a, _)| a)
}

/// `p`, затем `q`; значение — от `q`.
pub fn right<'a, A, B>(p: impl Parser<'a, A>, q: impl Parser<'a, B>) -> impl Parser<'a, B> {
    map(pair(p, q), |(_, b)| b)
}

/// `p`, а если не подошёл, `q` с того же места.
pub fn alt<'a, T>(p: impl Parser<'a, T>, q: impl Parser<'a, T>) -> impl Parser<'a, T> {
    move |s| p(s).or_else(|| q(s))
}

/// `p` столько раз, сколько подходит, хоть ни разу; значения сворачиваются
/// функцией `f`, начиная с `init`. Парсер, который ничего не съел,
/// останавливает повторение, иначе цикл был бы вечным.
pub fn many<'a, T, A: Copy>(
    p: impl Parser<'a, T>,
    init: A,
    f: impl Fn(A, T) -> A,
) -> impl Parser<'a, A> {
    move |mut s| {
        let mut acc = init;
        while let Some((t, rest)) = p(s) {
            if rest.len() == s.len() {
                break;
            }
            acc = f(acc, t);
            s = rest;
        }
        Some((acc, s))
    }
}
// ANCHOR_END: combinators

// ANCHOR: at
/// Ответ модема: данные и `OK`, просто `ERROR` или `+CME ERROR: <код>`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reply<T> {
    Data(T),
    Error,
    Cme(u32),
}

/// Строка ответа: содержимое и `\r\n`.
pub fn line<'a, T>(p: impl Parser<'a, T>) -> impl Parser<'a, T> {
    left(p, tag("\r\n"))
}

/// Ответ целиком: `\r\n`, затем данные, пустая строка и `OK` либо одна из
/// ошибок. Пустая строка отделяет `OK` от данных и бывает только после
/// них, поэтому она необязательна. Варианты `Reply` с полем — функции, и
/// `map` получает их прямо по имени.
pub fn reply<'a, T>(data: impl Parser<'a, T>) -> impl Parser<'a, Reply<T>> {
    let blank = alt(tag("\r\n"), tag(""));
    let ok = map(left(data, right(blank, line(tag("OK")))), Reply::Data);
    let error = map(line(tag("ERROR")), |_| Reply::Error);
    let cme = map(line(right(tag("+CME ERROR: "), number())), Reply::Cme);
    right(tag("\r\n"), alt(ok, alt(error, cme)))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Csq {
    pub rssi: u8,
    pub ber: u8,
}

/// `+CSQ: <rssi>,<ber>`: уровень 0–31 или 99 (неизвестен), ошибки 0–7
/// или 99.
pub fn csq<'a>() -> impl Parser<'a, Csq> {
    let level = |max: u32| and_then(number(), move |n| (n <= max || n == 99).then_some(n as u8));
    let body = pair(left(level(31), tag(",")), level(7));
    line(right(tag("+CSQ: "), map(body, |(rssi, ber)| Csq { rssi, ber })))
}

/// `+COPS: <mode>,<format>,"<оператор>"` — имя оператора.
pub fn cops<'a>() -> impl Parser<'a, &'a str> {
    let head = pair(left(number(), tag(",")), left(number(), tag(",")));
    line(right(tag("+COPS: "), right(head, quoted())))
}

/// Строки `+CMGL: <номер>,"<состояние>"` — номера сообщений битовой
/// маской; номер больше 31 — ошибка разбора.
pub fn cmgl<'a>() -> impl Parser<'a, u32> {
    let index = and_then(number(), |n| (n < 32).then(|| 1u32 << n));
    let entry = line(right(tag("+CMGL: "), left(index, right(tag(","), quoted()))));
    many(entry, 0, |mask, bit| mask | bit)
}

/// Весь ответ, без остатка.
fn complete<'a, T>(s: &'a str, p: impl Parser<'a, T>) -> Option<T> {
    match p(s)? {
        (t, "") => Some(t),
        _ => None,
    }
}

pub fn parse_csq(s: &str) -> Option<Reply<Csq>> {
    complete(s, reply(csq()))
}

pub fn parse_cops(s: &str) -> Option<Reply<&str>> {
    complete(s, reply(cops()))
}

pub fn parse_cmgl(s: &str) -> Option<Reply<u32>> {
    complete(s, reply(cmgl()))
}
// ANCHOR_END: at

// ANCHOR: probe
/// Для проверки в ассемблере: весь парсер встроен, вызовов по адресу нет.
#[unsafe(no_mangle)]
pub fn ch12_c_csq(s: &str) -> Option<Reply<Csq>> {
    parse_csq(s)
}

#[unsafe(no_mangle)]
pub fn ch12_c_cmgl(s: &str) -> Option<Reply<u32>> {
    parse_cmgl(s)
}
// ANCHOR_END: probe
