//! Задача B: разберите заголовок ELF64 двумя способами. Условие —
//! раздел 17.7.

/// Начало заголовка ELF64 (первые 32 байта). Все поля целые/байтовые.
#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Elf64Header {
    pub ident: [u8; 16],
    pub e_type: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u64,
}

// ANCHOR: skeleton
/// Без копирования: проверьте, что буфер не короче заголовка и выровнен
/// под `Elf64Header`, затем верните `&*(ptr as *const Elf64Header)` с
/// комментарием `// SAFETY`. Проверьте магию `\x7fELF`.
pub fn parse_zero_copy(buf: &[u8]) -> Option<&Elf64Header> {
    let _ = buf;
    todo!("раздел 17.7, задача B")
}

/// С копированием: соберите поля через `from_le_bytes`, без `unsafe` и без
/// требования выравнивания.
pub fn parse_safe(buf: &[u8]) -> Option<Elf64Header> {
    let _ = buf;
    todo!("раздел 17.7, задача B")
}
// ANCHOR_END: skeleton
