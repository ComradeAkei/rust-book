//! Задача B: разбор заголовка ELF64 двумя способами. Без копирования —
//! сырой срез переосмысляется как `&Elf64Header` после проверки размера и
//! выравнивания; с копированием — поля собираются `from_le_bytes`.
//! Первый способ не делает ни одного копирующего обращения и возвращает
//! ссылку внутрь буфера; второй переносим между порядками байт и не
//! требует ни `unsafe`, ни выравнивания.

/// Начало заголовка ELF64 (первые 32 байта). Все поля — целые и массивы
/// байт, поэтому любой битовый образец корректен: инварианта валидности
/// здесь нет, и переосмысление сырых байт как `Elf64Header` безопасно при
/// достаточном размере и выравнивании.
#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Elf64Header {
    pub ident: [u8; 16],
    pub e_type: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u64,
}

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

// ANCHOR: zero_copy
/// Переосмыслить начало буфера как `&Elf64Header` без копирования. Ссылка
/// живёт ровно столько, сколько сам буфер.
pub fn parse_zero_copy(buf: &[u8]) -> Option<&Elf64Header> {
    if buf.len() < size_of::<Elf64Header>() {
        return None;
    }
    let p = buf.as_ptr();
    if (p as usize) % align_of::<Elf64Header>() != 0 {
        return None; // невыровненный буфер — читать как Elf64Header нельзя
    }
    if buf[..4] != ELF_MAGIC {
        return None;
    }
    // SAFETY: размер проверен (буфер не короче заголовка), адрес выровнен
    // под Elf64Header, а все поля — целые/байтовые, для них любой образец
    // валиден. Ссылка привязана к времени жизни buf сигнатурой.
    Some(unsafe { &*(p as *const Elf64Header) })
}
// ANCHOR_END: zero_copy

// ANCHOR: safe
/// Собрать `Elf64Header` из байт через `from_le_bytes`: без `unsafe`, без
/// требования выравнивания, с явным little-endian.
pub fn parse_safe(buf: &[u8]) -> Option<Elf64Header> {
    let head = buf.get(..size_of::<Elf64Header>())?;
    if head[..4] != ELF_MAGIC {
        return None;
    }
    Some(Elf64Header {
        ident: head[..16].try_into().unwrap(),
        e_type: u16::from_le_bytes([head[16], head[17]]),
        machine: u16::from_le_bytes([head[18], head[19]]),
        version: u32::from_le_bytes(head[20..24].try_into().unwrap()),
        entry: u64::from_le_bytes(head[24..32].try_into().unwrap()),
    })
}
// ANCHOR_END: safe
