//! Задача C: тегированный указатель. `Node` выровнен на 8 байт, поэтому
//! три младших бита его адреса всегда нули — в них прячут тег состояния.
//! Работа идёт через строгий API provenance (`map_addr`, `addr`): право
//! обращаться к `Node` несёт исходный указатель, а тег — лишь число в
//! свободных битах. Проходит `-Zmiri-strict-provenance`.

/// Узел; выравнивание 8 освобождает три младших бита адреса под тег.
#[repr(align(8))]
#[derive(Debug, PartialEq)]
pub struct Node {
    pub value: u32,
}

const MASK: usize = 0b111;

// ANCHOR: tagged
/// Указатель на `Node` с тремя битами тега в младших разрядах адреса.
pub struct Tagged(*const Node);

impl Tagged {
    /// Запаковать указатель и тег (0..8). `map_addr` меняет только адрес,
    /// сохраняя provenance исходного указателя.
    pub fn new(p: *const Node, tag: usize) -> Self {
        debug_assert!(tag <= MASK);
        debug_assert!(p.addr() & MASK == 0, "указатель должен быть выровнен на 8");
        Tagged(p.map_addr(|a| a | (tag & MASK)))
    }

    /// Тег — это просто число из младших битов, provenance не нужно.
    pub fn tag(&self) -> usize {
        self.0.addr() & MASK
    }

    /// Очищенный указатель с тем же provenance — по нему можно обращаться
    /// к `Node`.
    pub fn ptr(&self) -> *const Node {
        self.0.map_addr(|a| a & !MASK)
    }
}
// ANCHOR_END: tagged
