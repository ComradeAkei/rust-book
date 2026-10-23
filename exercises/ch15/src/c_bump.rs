//! Задача C: bump-распределитель для синтаксического дерева формул
//! датчиков вида `(adc0 - 512) * 3 / 4 + -i2c1`.
//!
//! Разборщик, дерево из `Box` и генератор формул даны. Напишите `Bump` —
//! распределитель, который берёт память у системного кусками, растущими
//! вдвое, и раздаёт её подряд, сдвигая указатель, — и постройте на нём
//! второе дерево: `InBump` и `Expr::eval`. Узлы в `Bump` не уничтожаются
//! по одному, поэтому типы с `Drop` в него класть нельзя; пусть это
//! проверяет компилятор.
//!
//! Тесты проверяют и выравнивание, и блоки больше куска. Прогоните их под
//! Miri: он найдёт и выход за кусок, и потерянное происхождение указателя.
//! Затем сравните оба дерева по времени, памяти и системным вызовам
//! (`strace -c`) — условие в разделе 15.10.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

/// Распределитель-«указатель». Поля выберите сами.
pub struct Bump {}

impl Default for Bump {
    fn default() -> Self {
        Self::new()
    }
}

impl Bump {
    pub fn new() -> Self {
        todo!()
    }

    /// Положить значение в распределитель и отдать ссылку на него.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        todo!()
    }

    /// Копия строки в распределителе.
    pub fn alloc_str(&self, s: &str) -> &str {
        todo!()
    }

    /// Сколько кусков и сколько байт в них.
    pub fn footprint(&self) -> (usize, usize) {
        todo!()
    }
}

impl Drop for Bump {
    fn drop(&mut self) {
        // Освободите куски.
    }
}

// ANCHOR: ast
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Узел в `Bump`: дети — ссылки на узлы того же распределителя.
#[derive(Debug)]
pub enum Expr<'a> {
    Num(i64),
    Var(&'a str),
    Neg(&'a Expr<'a>),
    Bin(BinOp, &'a Expr<'a>, &'a Expr<'a>),
}

/// Тот же узел, каждый в своём `Box`.
#[derive(Debug)]
pub enum BoxExpr {
    Num(i64),
    Var(Box<str>),
    Neg(Box<BoxExpr>),
    Bin(BinOp, Box<BoxExpr>, Box<BoxExpr>),
}
// ANCHOR_END: ast

// ANCHOR: build
/// Как строить узлы: разборщик один, хранилище — параметр.
pub trait Build {
    type Node;
    fn num(&mut self, n: i64) -> Self::Node;
    fn var(&mut self, name: &str) -> Self::Node;
    fn neg(&mut self, e: Self::Node) -> Self::Node;
    fn bin(&mut self, op: BinOp, l: Self::Node, r: Self::Node) -> Self::Node;
}

pub struct Boxes;

impl Build for Boxes {
    type Node = Box<BoxExpr>;
    fn num(&mut self, n: i64) -> Self::Node {
        Box::new(BoxExpr::Num(n))
    }
    fn var(&mut self, name: &str) -> Self::Node {
        Box::new(BoxExpr::Var(name.into()))
    }
    fn neg(&mut self, e: Self::Node) -> Self::Node {
        Box::new(BoxExpr::Neg(e))
    }
    fn bin(&mut self, op: BinOp, l: Self::Node, r: Self::Node) -> Self::Node {
        Box::new(BoxExpr::Bin(op, l, r))
    }
}

pub struct InBump<'a>(pub &'a Bump);

impl<'a> Build for InBump<'a> {
    type Node = &'a Expr<'a>;
    fn num(&mut self, n: i64) -> Self::Node {
        todo!()
    }
    fn var(&mut self, name: &str) -> Self::Node {
        todo!()
    }
    fn neg(&mut self, e: Self::Node) -> Self::Node {
        todo!()
    }
    fn bin(&mut self, op: BinOp, l: Self::Node, r: Self::Node) -> Self::Node {
        todo!()
    }
}
// ANCHOR_END: build

// ANCHOR: parse
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Смещение в байтах, где разбор остановился.
    pub at: usize,
}

struct Parser<'s, B> {
    src: &'s [u8],
    at: usize,
    b: B,
}

/// Разбор формулы: `+ -` слабее `* /`, унарный минус, скобки, целые и
/// имена из букв, цифр и `_`.
pub fn parse<B: Build>(src: &str, b: B) -> Result<B::Node, ParseError> {
    let mut p = Parser { src: src.as_bytes(), at: 0, b };
    let e = p.expr()?;
    p.skip_ws();
    if p.at == p.src.len() { Ok(e) } else { Err(ParseError { at: p.at }) }
}

impl<B: Build> Parser<'_, B> {
    fn skip_ws(&mut self) {
        while self.src.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.src.get(self.at).copied()
    }

    fn expr(&mut self) -> Result<B::Node, ParseError> {
        let mut l = self.term()?;
        while let Some(c @ (b'+' | b'-')) = self.peek() {
            self.at += 1;
            let r = self.term()?;
            l = self.b.bin(if c == b'+' { BinOp::Add } else { BinOp::Sub }, l, r);
        }
        Ok(l)
    }

    fn term(&mut self) -> Result<B::Node, ParseError> {
        let mut l = self.factor()?;
        while let Some(c @ (b'*' | b'/')) = self.peek() {
            self.at += 1;
            let r = self.factor()?;
            l = self.b.bin(if c == b'*' { BinOp::Mul } else { BinOp::Div }, l, r);
        }
        Ok(l)
    }

    fn factor(&mut self) -> Result<B::Node, ParseError> {
        let err = ParseError { at: self.at };
        match self.peek() {
            Some(b'-') => {
                self.at += 1;
                let e = self.factor()?;
                Ok(self.b.neg(e))
            }
            Some(b'(') => {
                self.at += 1;
                let e = self.expr()?;
                if self.peek() != Some(b')') {
                    return Err(ParseError { at: self.at });
                }
                self.at += 1;
                Ok(e)
            }
            Some(c) if c.is_ascii_digit() => {
                let start = self.at;
                while self.src.get(self.at).is_some_and(u8::is_ascii_digit) {
                    self.at += 1;
                }
                let s = std::str::from_utf8(&self.src[start..self.at]).unwrap();
                s.parse().map(|n| self.b.num(n)).map_err(|_| ParseError { at: start })
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.at;
                while self.src.get(self.at).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    self.at += 1;
                }
                let s = std::str::from_utf8(&self.src[start..self.at]).unwrap();
                Ok(self.b.var(s))
            }
            _ => Err(err),
        }
    }
}
// ANCHOR_END: parse

// ANCHOR: eval
fn apply(op: BinOp, a: i64, b: i64) -> i64 {
    match op {
        BinOp::Add => a.wrapping_add(b),
        BinOp::Sub => a.wrapping_sub(b),
        BinOp::Mul => a.wrapping_mul(b),
        // Деление на ноль даёт ноль: формула датчика не должна ронять станцию.
        BinOp::Div => a.checked_div(b).unwrap_or(0),
    }
}

impl Expr<'_> {
    pub fn eval(&self, vars: &impl Fn(&str) -> i64) -> i64 {
        todo!()
    }
}

impl BoxExpr {
    pub fn eval(&self, vars: &impl Fn(&str) -> i64) -> i64 {
        match self {
            BoxExpr::Num(n) => *n,
            BoxExpr::Var(v) => vars(v),
            BoxExpr::Neg(e) => e.eval(vars).wrapping_neg(),
            BoxExpr::Bin(op, l, r) => apply(*op, l.eval(vars), r.eval(vars)),
        }
    }
}
// ANCHOR_END: eval

// ANCHOR: gen
/// Случайная формула примерно из `nodes` узлов. Дерево сбалансировано:
/// глубина растёт как логарифм, и рекурсия разбора, вычисления и `drop` у
/// `Box` не переполняет стек.
pub fn gen_source(nodes: usize, seed: u64) -> String {
    fn go(out: &mut String, nodes: usize, s: &mut u64) {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        let r = *s;
        if nodes <= 1 {
            if r.is_multiple_of(3) {
                out.push_str(["adc0", "adc1", "i2c_t", "spi2"][(r >> 8) as usize % 4]);
            } else {
                out.push_str(&((r >> 8) % 1000).to_string());
            }
        } else if r.is_multiple_of(16) {
            out.push_str("-(");
            go(out, nodes - 1, s);
            out.push(')');
        } else {
            let left = (nodes - 1) / 2;
            out.push('(');
            go(out, left, s);
            out.push_str([" + ", " - ", " * ", " / "][(r >> 8) as usize % 4]);
            go(out, nodes - 1 - left, s);
            out.push(')');
        }
    }
    let mut out = String::new();
    let mut s = seed | 1;
    go(&mut out, nodes.max(1), &mut s);
    out
}
// ANCHOR_END: gen
