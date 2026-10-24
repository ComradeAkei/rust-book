//! Задача A: таблица векторов из ELF прошивки. Скрипт компоновки
//! cortex-m-rt кладёт по адресу 0 секцию `.vector_table`: слово 0 —
//! начальное значение SP, дальше адреса обработчиков исключений и
//! прерываний. Адрес обработчика — с битом 0: процессор переходит по
//! нему, и бит 0 становится битом T регистра EPSR.

// ANCHOR: api
/// Таблица векторов: адрес секции и её байты из файла.
#[derive(Clone, Copy, Debug)]
pub struct Vectors<'a> {
    /// Адрес секции в памяти программы (`sh_addr`).
    pub addr: u32,
    bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VtError {
    /// Не ELF, не 32 бита или не little-endian.
    NotElf32,
    /// Таблица заголовков секций или строк выходит за файл.
    Truncated,
    /// Секции `.vector_table` нет.
    NoTable,
    /// Размер секции не кратен слову или меньше 16 слов.
    BadSize(u32),
    /// Начальный SP не выровнен на 8 байт.
    StackNotAligned(u32),
    /// Адрес обработчика без бита Thumb.
    EvenHandler { index: usize, addr: u32 },
}
// ANCHOR_END: api

fn u16_at(b: &[u8], off: usize) -> Result<u16, VtError> {
    let s = b.get(off..off + 2).ok_or(VtError::Truncated)?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], off: usize) -> Result<u32, VtError> {
    let s = b.get(off..off + 4).ok_or(VtError::Truncated)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Заголовок секции ELF32: имя (смещение в таблице строк), адрес, место в
/// файле и размер.
struct Section {
    name: u32,
    addr: u32,
    offset: u32,
    size: u32,
}

fn section(elf: &[u8], shoff: usize, shentsize: usize, i: usize) -> Result<Section, VtError> {
    let h = shoff + i * shentsize;
    Ok(Section {
        name: u32_at(elf, h)?,
        addr: u32_at(elf, h + 12)?,
        offset: u32_at(elf, h + 16)?,
        size: u32_at(elf, h + 20)?,
    })
}

/// Байты секции из файла.
fn bytes<'a>(elf: &'a [u8], s: &Section) -> Result<&'a [u8], VtError> {
    let start = s.offset as usize;
    elf.get(start..start + s.size as usize).ok_or(VtError::Truncated)
}

// ANCHOR: parse
/// Найти `.vector_table` по таблице заголовков секций и её таблице имён
/// (`e_shstrndx`).
pub fn vector_table(elf: &[u8]) -> Result<Vectors<'_>, VtError> {
    // ELFCLASS32 = 1, ELFDATA2LSB = 1.
    if elf.get(..6) != Some(b"\x7fELF\x01\x01".as_slice()) {
        return Err(VtError::NotElf32);
    }
    let shoff = u32_at(elf, 0x20)? as usize;
    let shentsize = usize::from(u16_at(elf, 0x2E)?);
    let shnum = usize::from(u16_at(elf, 0x30)?);
    let shstrndx = usize::from(u16_at(elf, 0x32)?);
    let names = bytes(elf, &section(elf, shoff, shentsize, shstrndx)?)?;
    for i in 0..shnum {
        let s = section(elf, shoff, shentsize, i)?;
        let name = names.get(s.name as usize..).ok_or(VtError::Truncated)?;
        if !name.starts_with(b".vector_table\0") {
            continue;
        }
        if s.size % 4 != 0 || s.size < 16 * 4 {
            return Err(VtError::BadSize(s.size));
        }
        return Ok(Vectors { addr: s.addr, bytes: bytes(elf, &s)? });
    }
    Err(VtError::NoTable)
}
// ANCHOR_END: parse

impl Vectors<'_> {
    /// Число слов, начальный SP включительно.
    pub fn len(&self) -> usize {
        self.bytes.len() / 4
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Слово `i`: 0 — начальный SP, 1 — Reset, 16 и дальше — прерывания.
    pub fn get(&self, i: usize) -> Option<u32> {
        let s = self.bytes.get(i * 4..i * 4 + 4)?;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    /// Число прерываний после 16 слов ядра.
    pub fn interrupts(&self) -> usize {
        self.len() - 16
    }

    // ANCHOR: check
    /// Начальный SP выровнен на 8 байт (AAPCS), у каждого ненулевого
    /// адреса обработчика бит 0 — бит Thumb. Нули — зарезервированные
    /// слова.
    pub fn check(&self) -> Result<(), VtError> {
        let sp = self.get(0).unwrap_or(0);
        if sp % 8 != 0 {
            return Err(VtError::StackNotAligned(sp));
        }
        for index in 1..self.len() {
            let addr = self.get(index).unwrap_or(0);
            if addr != 0 && addr & 1 == 0 {
                return Err(VtError::EvenHandler { index, addr });
            }
        }
        Ok(())
    }
    // ANCHOR_END: check
}
