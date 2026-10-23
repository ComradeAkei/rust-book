//! Задача C: тегированный указатель — три бита состояния в младших
//! разрядах выровненного на 8 адреса. Работайте через `map_addr`/`addr`
//! (строгий provenance). Условие — раздел 17.7.

/// Узел; выравнивание 8 освобождает три младших бита адреса под тег.
#[repr(align(8))]
#[derive(Debug, PartialEq)]
pub struct Node {
    pub value: u32,
}

// ANCHOR: skeleton
// Поле держит упакованный указатель; в заготовке оно ещё не используется.
#[allow(dead_code)]
pub struct Tagged(*const Node);

impl Tagged {
    /// Запаковать `p` (выровнен на 8) и `tag` (0..8) через `map_addr`.
    pub fn new(p: *const Node, tag: usize) -> Self {
        let _ = (p, tag);
        todo!("раздел 17.7, задача C")
    }

    /// Младшие три бита адреса.
    pub fn tag(&self) -> usize {
        todo!("раздел 17.7, задача C")
    }

    /// Очищенный указатель с тем же provenance.
    pub fn ptr(&self) -> *const Node {
        todo!("раздел 17.7, задача C")
    }
}
// ANCHOR_END: skeleton
