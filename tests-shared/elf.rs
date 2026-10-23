// Разбор ELF (32 и 64 бита, little-endian) и архивов ar: символы с
// адресом и размером. Хватает и для объектных файлов в .rlib,
// и для собранной программы.
//
// Файл подключают через `include!` внутри `mod elf { … }`: общие тесты
// главы 10 и тесты `code/ch10/tests`.

pub struct Symbol {
    pub name: String,
    pub value: u64,
    pub size: u64,
    /// Номер секции: 0 — символ не определён здесь.
    pub shndx: usize,
    pub func: bool,
}

pub struct Elf {
    pub symbols: Vec<Symbol>,
}

fn u(b: &[u8], at: usize, size: usize) -> u64 {
    b[at..at + size].iter().rev().fold(0, |v, &x| v << 8 | u64::from(x))
}

fn cstr(b: &[u8], at: usize) -> String {
    let end = b[at..].iter().position(|&c| c == 0).unwrap();
    String::from_utf8_lossy(&b[at..at + end]).into_owned()
}

pub fn parse(b: &[u8]) -> Elf {
    assert!(b.starts_with(b"\x7fELF") && b[5] == 1, "ожидался ELF little-endian");
    let wide = b[4] == 2;
    // Смещения полей заголовков ELF32 и ELF64.
    let (shoff, shentsize, shnum) = if wide {
        (u(b, 0x28, 8), u(b, 0x3a, 2), u(b, 0x3c, 2))
    } else {
        (u(b, 0x20, 4), u(b, 0x2e, 2), u(b, 0x30, 2))
    };
    let (shoff, shentsize, shnum) = (shoff as usize, shentsize as usize, shnum as usize);
    // Поле f заголовка секции i: тип, смещение, размер, связь.
    let sh = |i: usize, f: &str| {
        let base = shoff + i * shentsize;
        let (at, size) = match (f, wide) {
            ("type", _) => (4, 4),
            ("offset", true) => (0x18, 8),
            ("size", true) => (0x20, 8),
            ("link", true) => (0x28, 4),
            ("offset", false) => (0x10, 4),
            ("size", false) => (0x14, 4),
            ("link", false) => (0x18, 4),
            _ => unreachable!(),
        };
        u(b, base + at, size) as usize
    };
    let mut symbols = Vec::new();
    let entsize = if wide { 24 } else { 16 };
    for i in (0..shnum).filter(|&i| sh(i, "type") == 2) {
        let strtab = sh(sh(i, "link"), "offset");
        let start = sh(i, "offset");
        for s in (start..start + sh(i, "size")).step_by(entsize).skip(1) {
            let (info, shndx, value, size) = if wide {
                (b[s + 4], u(b, s + 6, 2), u(b, s + 8, 8), u(b, s + 16, 8))
            } else {
                (b[s + 12], u(b, s + 14, 2), u(b, s + 4, 4), u(b, s + 8, 4))
            };
            symbols.push(Symbol {
                name: cstr(b, strtab + u(b, s, 4) as usize),
                value,
                size,
                shndx: shndx as usize,
                func: info & 0xf == 2,
            });
        }
    }
    Elf { symbols }
}

/// Члены архива ar в формате GNU, кроме служебных.
pub fn ar(b: &[u8]) -> Vec<&[u8]> {
    assert!(b.starts_with(b"!<arch>\n"), "не архив ar");
    let mut pos = 8;
    let mut out = Vec::new();
    while pos + 60 <= b.len() {
        let size: usize =
            String::from_utf8_lossy(&b[pos + 48..pos + 58]).trim().parse().unwrap();
        out.push(&b[pos + 60..pos + 60 + size]);
        pos += 60 + size + size % 2;
    }
    out
}

/// Все символы объектных файлов из .rlib.
pub fn rlib_symbols(bytes: &[u8]) -> Vec<Symbol> {
    ar(bytes)
        .into_iter()
        .filter(|m| m.starts_with(b"\x7fELF"))
        .flat_map(|m| parse(m).symbols)
        .collect()
}
