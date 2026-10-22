//! Задача C, вторая часть: разбор имён символов Rust. Схема legacy
//! (`_ZN…E`, совместимая с C++) разбирается целиком, схема v0 (`_R…`) — в
//! подмножестве из условия: пути, реализации, обобщённые аргументы,
//! замыкания, ссылки на повторы и типы без `dyn` и указателей на функции.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

// ANCHOR: demangle
/// Имя символа без хешей и дизамбигуаторов, как его печатает
/// `llvm-cxxfilt`. `None`, если это не имя Rust или в нём встретилось
/// что-то вне подмножества.
pub fn demangle(sym: &str) -> Option<String> {
    match sym.strip_prefix("_R").or_else(|| sym.strip_prefix("__R")) {
        Some(rest) => v0(rest),
        None => legacy(sym),
    }
}
// ANCHOR_END: demangle

// ---------------------------------------------------------------- legacy

// ANCHOR: legacy
/// `_ZN` {длина имя} `E`: компоненты пути как в C++, последний — хеш
/// `h` + 16 шестнадцатеричных цифр. Символы, недопустимые в именах
/// ассемблера, закодированы: `$LT$` — `<`, `..` — `::` и так далее.
fn legacy(sym: &str) -> Option<String> {
    let mut rest = ["_ZN", "__ZN", "ZN"].iter().find_map(|p| sym.strip_prefix(p))?;
    let mut parts = Vec::new();
    while let Some(c) = rest.bytes().next() {
        if c == b'E' {
            break;
        }
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let len: usize = rest[..digits].parse().ok()?;
        let part = rest.get(digits..digits + len)?;
        parts.push(part);
        rest = &rest[digits + len..];
    }
    // После E допустим только суффикс LLVM вроде `.llvm.1234`.
    let tail = rest.strip_prefix('E')?;
    if !(tail.is_empty() || tail.starts_with('.')) {
        return None;
    }
    if parts.last().is_some_and(|h| is_hash(h)) {
        parts.pop();
    }
    if parts.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str("::");
        }
        unescape(part, &mut out)?;
    }
    Some(out)
}

fn is_hash(s: &str) -> bool {
    s.len() == 17 && s.starts_with('h') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn unescape(part: &str, out: &mut String) -> Option<()> {
    // Имя, которое начиналось бы с `$`, получает спереди `_`.
    let mut p = part.strip_prefix("_$").map_or(part, |_| &part[1..]);
    while !p.is_empty() {
        if let Some(r) = p.strip_prefix("..") {
            out.push_str("::");
            p = r;
        } else if let Some(r) = p.strip_prefix('$') {
            let end = r.find('$')?;
            let c = match &r[..end] {
                "SP" => '@',
                "BP" => '*',
                "RF" => '&',
                "LT" => '<',
                "GT" => '>',
                "LP" => '(',
                "RP" => ')',
                "C" => ',',
                code => char::from_u32(u32::from_str_radix(code.strip_prefix('u')?, 16).ok()?)?,
            };
            out.push(c);
            p = &r[end + 1..];
        } else {
            let n = p.char_indices().skip(1).find(|&(_, c)| c == '$' || c == '.');
            let n = n.map_or(p.len(), |(n, _)| n);
            out.push_str(&p[..n]);
            p = &p[n..];
        }
    }
    Some(())
}
// ANCHOR_END: legacy

// ---------------------------------------------------------------- v0

// ANCHOR: v0
// ANCHOR: v0-entry
/// Схема v0 (RFC 2603): `_R` путь [крейт конкретизации] [.суффикс].
fn v0(rest: &str) -> Option<String> {
    // Суффикс после точки добавляют LLVM и rustc (`.llvm.1234`, `.0`); в
    // самом имени v0 точек нет. Цифра в начале была бы версией схемы, но
    // rustc 1.97 её не пишет, а путь с цифры не начинается.
    let (body, suffix) = rest.split_at(rest.find('.').unwrap_or(rest.len()));
    let mut p = Parser { sym: body.as_bytes(), pos: 0, depth: 0, out: String::new() };
    p.path(true)?;
    // У конкретизаций обобщённого кода в конце стоит путь крейта, где они
    // созданы. Печатать его не нужно, но разобрать — да.
    if p.pos < p.sym.len() {
        p.skip(|p| p.path(false))?;
    }
    if p.pos != p.sym.len() {
        return None;
    }
    Some(with_suffix(p.out, suffix))
}

/// Суффикс печатается в скобках, как у `llvm-cxxfilt`.
fn with_suffix(mut out: String, suffix: &str) -> String {
    if !suffix.is_empty() {
        out.push_str(" (");
        out.push_str(suffix);
        out.push(')');
    }
    out
}
// ANCHOR_END: v0-entry

// ANCHOR: v0-lexer
struct Parser<'s> {
    /// Имя без `_R`: от его начала отсчитываются ссылки на повторы `B`.
    sym: &'s [u8],
    pos: usize,
    depth: u32,
    out: String,
}

impl<'s> Parser<'s> {
    fn peek(&self) -> Option<u8> {
        self.sym.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn eat(&mut self, b: u8) -> bool {
        let yes = self.peek() == Some(b);
        self.pos += usize::from(yes);
        yes
    }

    /// Защита от переполнения стека на испорченном имени.
    fn enter(&mut self) -> Option<()> {
        self.depth += 1;
        (self.depth < 300).then_some(())
    }

    /// Разобрать, но ничего не напечатать.
    fn skip(&mut self, f: impl FnOnce(&mut Self) -> Option<()>) -> Option<()> {
        let len = self.out.len();
        f(self)?;
        self.out.truncate(len);
        Some(())
    }

    /// Число в base-62, закрытое `_`: `_` — 0, `0_` — 1, `Z_` — 62.
    fn base62(&mut self) -> Option<u64> {
        let mut x: u64 = 0;
        if self.eat(b'_') {
            return Some(0);
        }
        loop {
            let d = match self.next()? {
                b @ b'0'..=b'9' => b - b'0',
                b @ b'a'..=b'z' => b - b'a' + 10,
                b @ b'A'..=b'Z' => b - b'A' + 36,
                b'_' => return x.checked_add(1),
                _ => return None,
            };
            x = x.checked_mul(62)?.checked_add(u64::from(d))?;
        }
    }

    /// `s` base-62: номер среди одноимённых; без него — 0.
    fn disambiguator(&mut self) -> Option<u64> {
        if self.eat(b's') { self.base62()?.checked_add(1) } else { Some(0) }
    }

    /// Длина в десятичной записи, необязательный `_` и сами байты.
    fn ident(&mut self) -> Option<&'s str> {
        if self.peek() == Some(b'u') {
            return None; // Punycode для не-ASCII имён: вне подмножества.
        }
        let start = self.pos;
        // Число без ведущих нулей: «00» — это два пустых имени подряд.
        if !self.eat(b'0') {
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let len: usize = core::str::from_utf8(&self.sym[start..self.pos]).ok()?.parse().ok()?;
        self.eat(b'_');
        let bytes = self.sym.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        core::str::from_utf8(bytes).ok()
    }

    /// `B` base-62: повтор того, что уже встретилось раньше в имени.
    fn backref(&mut self, f: impl FnOnce(&mut Self) -> Option<()>) -> Option<()> {
        let at = self.pos - 1;
        let target = usize::try_from(self.base62()?).ok()?;
        if target >= at {
            return None;
        }
        let back = core::mem::replace(&mut self.pos, target);
        f(self)?;
        self.pos = back;
        Some(())
    }
    // ANCHOR_END: v0-lexer

    // ANCHOR: v0-path
    /// Путь. В пути значения обобщённые аргументы пишутся через `::<>`,
    /// в типе — через `<>`.
    fn path(&mut self, in_value: bool) -> Option<()> {
        self.enter()?;
        match self.next()? {
            b'C' => {
                self.disambiguator()?;
                let name = self.ident()?;
                self.out.push_str(name);
            }
            // Собственная реализация: <T>. Путь самого impl не печатается.
            b'M' => {
                self.skip(|p| p.impl_path())?;
                self.out.push('<');
                self.ty()?;
                self.out.push('>');
            }
            // Реализация трейта (X) или путь через трейт (Y): <T as Trait>.
            tag @ (b'X' | b'Y') => {
                if tag == b'X' {
                    self.skip(|p| p.impl_path())?;
                }
                self.out.push('<');
                self.ty()?;
                self.out.push_str(" as ");
                self.path(false)?;
                self.out.push('>');
            }
            b'N' => {
                let ns = self.next()?;
                self.path(in_value)?;
                let dis = self.disambiguator()?;
                let name = self.ident()?;
                match ns {
                    // Конструктор (`c`) и прочие служебные имена бывают пустыми.
                    b'a'..=b'z' if name.is_empty() => {}
                    b'a'..=b'z' => {
                        self.out.push_str("::");
                        self.out.push_str(name);
                    }
                    // Безымянные сущности: замыкания, прокладки (shim).
                    b'A'..=b'Z' => {
                        self.out.push_str("::{");
                        match ns {
                            b'C' => self.out.push_str("closure"),
                            b'S' => self.out.push_str("shim"),
                            _ => self.out.push(char::from(ns)),
                        }
                        if !name.is_empty() {
                            self.out.push(':');
                            self.out.push_str(name);
                        }
                        write!(self.out, "#{dis}}}").ok()?;
                    }
                    _ => return None,
                }
            }
            b'I' => {
                self.path(in_value)?;
                if in_value {
                    self.out.push_str("::");
                }
                self.out.push('<');
                let mut first = true;
                while !self.eat(b'E') {
                    if !first {
                        self.out.push_str(", ");
                    }
                    first = false;
                    self.generic_arg()?;
                }
                self.out.push('>');
            }
            b'B' => self.backref(|p| p.path(in_value))?,
            _ => return None,
        }
        self.depth -= 1;
        Some(())
    }

    fn impl_path(&mut self) -> Option<()> {
        self.disambiguator()?;
        self.path(false)
    }

    fn generic_arg(&mut self) -> Option<()> {
        if self.eat(b'L') {
            // Стёртое время жизни. Именованные (через связыватель G) вне
            // подмножества.
            (self.base62()? == 0).then(|| self.out.push_str("'_"))
        } else if self.eat(b'K') {
            self.konst()
        } else {
            self.ty()
        }
    }
    // ANCHOR_END: v0-path

    // ANCHOR: v0-ty
    fn ty(&mut self) -> Option<()> {
        self.enter()?;
        let tag = self.next()?;
        if let Some(name) = basic(tag) {
            self.out.push_str(name);
        } else {
            match tag {
                b'R' | b'Q' => {
                    self.out.push('&');
                    if self.eat(b'L') && self.base62()? != 0 {
                        return None;
                    }
                    if tag == b'Q' {
                        self.out.push_str("mut ");
                    }
                    self.ty()?;
                }
                b'P' | b'O' => {
                    self.out.push_str(if tag == b'P' { "*const " } else { "*mut " });
                    self.ty()?;
                }
                b'S' | b'A' => {
                    self.out.push('[');
                    self.ty()?;
                    if tag == b'A' {
                        self.out.push_str("; ");
                        self.konst()?;
                    }
                    self.out.push(']');
                }
                b'T' => {
                    self.out.push('(');
                    let mut n = 0;
                    while !self.eat(b'E') {
                        if n > 0 {
                            self.out.push_str(", ");
                        }
                        self.ty()?;
                        n += 1;
                    }
                    if n == 1 {
                        self.out.push(',');
                    }
                    self.out.push(')');
                }
                b'B' => self.backref(|p| p.ty())?,
                b'C' | b'M' | b'X' | b'Y' | b'N' | b'I' => {
                    self.pos -= 1;
                    self.path(false)?;
                }
                _ => return None,
            }
        }
        self.depth -= 1;
        Some(())
    }

    /// Константа обобщённого аргумента: тип и значение в шестнадцатеричной
    /// записи, закрытое `_`; `n` перед числом — минус.
    fn konst(&mut self) -> Option<()> {
        match self.next()? {
            b'p' => self.out.push('_'),
            b'B' => self.backref(|p| p.konst())?,
            ty => {
                let neg = matches!(ty, b'a' | b's' | b'l' | b'x' | b'n' | b'i') && self.eat(b'n');
                let start = self.pos;
                while self.peek()?.is_ascii_hexdigit() {
                    self.pos += 1;
                }
                let hex = core::str::from_utf8(&self.sym[start..self.pos]).ok()?;
                if !self.eat(b'_') {
                    return None;
                }
                let v = if hex.is_empty() { 0 } else { u128::from_str_radix(hex, 16).ok()? };
                match ty {
                    b'h' | b't' | b'm' | b'y' | b'o' | b'j' => write!(self.out, "{v}").ok()?,
                    b'a' | b's' | b'l' | b'x' | b'n' | b'i' => {
                        write!(self.out, "{}{v}", if neg { "-" } else { "" }).ok()?
                    }
                    b'b' if v < 2 => self.out.push_str(if v == 1 { "true" } else { "false" }),
                    b'c' => {
                        write!(self.out, "{:?}", char::from_u32(u32::try_from(v).ok()?)?).ok()?
                    }
                    _ => return None,
                }
            }
        }
        Some(())
    }
}

fn basic(tag: u8) -> Option<&'static str> {
    Some(match tag {
        b'a' => "i8",
        b'b' => "bool",
        b'c' => "char",
        b'd' => "f64",
        b'e' => "str",
        b'f' => "f32",
        b'h' => "u8",
        b'i' => "isize",
        b'j' => "usize",
        b'l' => "i32",
        b'm' => "u32",
        b'n' => "i128",
        b'o' => "u128",
        b's' => "i16",
        b't' => "u16",
        b'u' => "()",
        b'v' => "...",
        b'x' => "i64",
        b'y' => "u64",
        b'z' => "!",
        b'p' => "_",
        _ => return None,
    })
}
// ANCHOR_END: v0-ty
// ANCHOR_END: v0
