//! 24.2: `#[derive(Describe)]` — описание раскладки структуры, которое
//! пишет за человека процедурный макрос: имя, тип, смещение и размер
//! каждого поля и дамп значений. 24.4: атрибуты-помощники
//! `#[describe(size = N)]`, `#[describe(skip)]` и `#[describe(crate = путь)]`.
//!
//! Ошибки, которые derive находит сам, указывают в код пользователя:
//!
//! ```compile_fail
//! #[derive(ch24::Describe)]
//! enum Mode { Off, On }
//! ```
//!
//! Размер, обещанный в атрибуте, проверяется при компиляции — E0080:
//!
//! ```compile_fail,E0080
//! #[derive(ch24::Describe)]
//! #[describe(size = 8)]
//! #[repr(C)]
//! struct Frame { id: u32, len: u32, crc: u32 }
//! ```
//!
//! Поле без `Debug` не даёт собрать `dump` — E0277:
//!
//! ```compile_fail,E0277
//! struct Opaque;
//! #[derive(ch24::Describe)]
//! struct Holder { raw: Opaque }
//! ```
//!
//! а с `#[describe(skip)]` поле пропускается:
//!
//! ```
//! struct Opaque;
//! #[derive(ch24::Describe)]
//! struct Holder { id: u8, #[describe(skip)] _raw: Opaque }
//! assert_eq!(<Holder as ch24::Describe>::FIELDS.len(), 1);
//! ```

use core::fmt;

// ANCHOR: trait
/// Одно поле: имя, тип текстом, смещение от начала структуры и размер.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub ty: &'static str,
    pub offset: usize,
    pub size: usize,
}

/// Описание раскладки. Реализацию пишет `#[derive(Describe)]`.
pub trait Describe {
    const NAME: &'static str;
    const SIZE: usize;
    const ALIGN: usize;
    /// Поля в порядке объявления (не в порядке в памяти).
    const FIELDS: &'static [Field];

    /// Дамп значения: смещение, имя, тип и `Debug` каждого поля.
    fn dump(&self, out: &mut dyn fmt::Write) -> fmt::Result;
}
// ANCHOR_END: trait

/// Первая строка дампа. Вызывается из сгенерированного `dump`.
pub fn dump_header<T: Describe + ?Sized>(out: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(out, "{} ({} байт)", T::NAME, T::SIZE)
}

/// Строка дампа для одного поля. Сгенерированный код передаёт сюда
/// `&self.поле` как `&dyn Debug`: поле без `Debug` — ошибка E0277.
pub fn dump_field(out: &mut dyn fmt::Write, f: &Field, value: &dyn fmt::Debug) -> fmt::Result {
    writeln!(out, "  +{:<2} {}: {} = {:?}", f.offset, f.name, f.ty, value)
}

/// Таблица раскладки по возрастанию смещений, с дырами выравнивания.
pub fn layout<T: Describe + ?Sized>() -> impl fmt::Display {
    Layout::<T>(core::marker::PhantomData)
}

struct Layout<T: ?Sized>(core::marker::PhantomData<T>);

impl<T: Describe + ?Sized> fmt::Display for Layout<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {} байт, выравнивание {}", T::NAME, T::SIZE, T::ALIGN)?;
        let mut at = 0;
        // Выбор следующего по смещению поля: без кучи, полей немного.
        // Поля нулевого размера идут раньше соседей с тем же смещением.
        let mut done = 0;
        let mut last: Option<(usize, usize, usize)> = None;
        while done < T::FIELDS.len() {
            let key = |i: usize, fl: &Field| (fl.offset, fl.size, i);
            let next = T::FIELDS
                .iter()
                .enumerate()
                .map(|(i, fl)| key(i, fl))
                .filter(|k| last.is_none_or(|l| k > &l))
                .min()
                .unwrap();
            let fl = &T::FIELDS[next.2];
            if fl.offset > at {
                writeln!(f, "  +{:<2} [дыра {}]", at, fl.offset - at)?;
            }
            writeln!(f, "  +{:<2} {}: {} ({})", fl.offset, fl.name, fl.ty, fl.size)?;
            at = at.max(fl.offset + fl.size);
            last = Some(next);
            done += 1;
        }
        if T::SIZE > at {
            writeln!(f, "  +{:<2} [дыра {}]", at, T::SIZE - at)?;
        }
        Ok(())
    }
}

// ANCHOR: header
/// Заголовок кадра телеметрии: между `kind` и `crc` — дыра выравнивания.
#[derive(super::Describe, Debug, Clone, Copy)]
#[describe(size = 8)]
#[repr(C)]
pub struct Header {
    pub magic: u16,
    pub kind: u8,
    pub crc: u32,
}
// ANCHOR_END: header

// ANCHOR: generic
/// Отсчёт датчика с меткой времени: обобщённый тип и кортежная структура.
#[derive(super::Describe, Debug)]
#[repr(C)]
pub struct Sample<T> {
    pub tick: u32,
    pub value: T,
}

#[derive(super::Describe, Debug)]
#[repr(C)]
pub struct Pair(pub u8, pub u16);
// ANCHOR_END: generic

/// Те же три поля, что у [`Unordered`], но в раскладке C.
#[derive(super::Describe)]
#[repr(C)]
pub struct Ordered {
    pub a: u8,
    pub b: u32,
    pub c: u8,
}

/// Раскладка Rust: порядок полей в памяти выбирает компилятор.
#[derive(super::Describe)]
pub struct Unordered {
    pub a: u8,
    pub b: u32,
    pub c: u8,
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::Describe;

    // ANCHOR: header_test
    #[test]
    fn header_fields() {
        assert_eq!(Header::NAME, "Header");
        assert_eq!((Header::SIZE, Header::ALIGN), (8, 4));
        let f = Header::FIELDS;
        assert_eq!(f.iter().map(|f| f.name).collect::<Vec<_>>(), ["magic", "kind", "crc"]);
        assert_eq!(f.iter().map(|f| f.offset).collect::<Vec<_>>(), [0, 2, 4]);
        assert_eq!(f.iter().map(|f| f.size).collect::<Vec<_>>(), [2, 1, 4]);
        assert_eq!(f[2], Field { name: "crc", ty: "u32", offset: 4, size: 4 });
    }

    #[test]
    fn header_layout_and_dump() {
        assert_eq!(
            layout::<Header>().to_string(),
            "\
Header: 8 байт, выравнивание 4
  +0  magic: u16 (2)
  +2  kind: u8 (1)
  +3  [дыра 1]
  +4  crc: u32 (4)
"
        );
        let h = Header { magic: 0x7E57, kind: 3, crc: 0xDEAD_BEEF };
        let mut s = String::new();
        h.dump(&mut s).unwrap();
        assert_eq!(
            s,
            "\
Header (8 байт)
  +0  magic: u16 = 32343
  +2  kind: u8 = 3
  +4  crc: u32 = 3735928559
"
        );
    }
    // ANCHOR_END: header_test

    #[test]
    fn generic_and_tuple() {
        assert_eq!(<Sample<u8>>::SIZE, 8);
        assert_eq!(<Sample<u64>>::FIELDS[1], Field { name: "value", ty: "T", offset: 8, size: 8 });
        assert_eq!(<Sample<u16>>::FIELDS[1], Field { name: "value", ty: "T", offset: 4, size: 2 });
        assert_eq!(
            Pair::FIELDS,
            [
                Field { name: "0", ty: "u8", offset: 0, size: 1 },
                Field { name: "1", ty: "u16", offset: 2, size: 2 }
            ]
        );
        let mut s = String::new();
        Sample { tick: 7, value: [1u8, 2] }.dump(&mut s).unwrap();
        assert_eq!(s, "Sample (8 байт)\n  +0  tick: u32 = 7\n  +4  value: T = [1, 2]\n");
    }

    #[test]
    fn type_text_comes_from_stringify() {
        #[derive(crate::Describe, Debug)]
        #[repr(C)]
        struct Buf<'a> {
            raw: [u8; 4],
            tail: &'a [u16],
            next: Option<&'a Buf<'a>>,
        }
        let tys: Vec<String> =
            Buf::FIELDS.iter().map(|f| f.ty.split_whitespace().collect()).collect();
        assert_eq!(tys, ["[u8;4]", "&'a[u16]", "Option<&'aBuf<'a>>"]);
    }

    /// Наблюдение rustc 1.97: токены, прошедшие через процедурный макрос,
    /// stringify! печатает с пробелами там, где в исходнике их не было.
    #[test]
    fn observed_stringify_spacing() {
        #[derive(crate::Describe, Debug)]
        #[repr(C)]
        struct Buf<'a> {
            raw: [u8; 4],
            tail: &'a [u16],
            next: Option<&'a Buf<'a>>,
        }
        let tys: Vec<_> = Buf::FIELDS.iter().map(|f| f.ty).collect();
        assert_eq!(tys, ["[u8; 4]", "& 'a [u16]", "Option < & 'a Buf < 'a > >"]);
    }

    // ANCHOR: hostile
    /// Код из derive раскрывается в модуле пользователя. Свои `core`,
    /// `Option`, `Result`, `fmt` и даже `ch24` ему не мешают: пути в нём
    /// начинаются с `::`.
    mod hostile {
        #![allow(dead_code, non_camel_case_types)]
        mod core {}
        mod fmt {}
        mod ch24 {}
        struct Option;
        struct Result;
        struct usize;

        #[derive(crate::Describe)]
        #[describe(size = 2)]
        #[repr(C)]
        pub struct Reg {
            pub lo: u8,
            pub hi: u8,
        }
    }

    #[test]
    fn hostile_module() {
        assert_eq!(<hostile::Reg as Describe>::FIELDS[1].offset, 1);
    }
    // ANCHOR_END: hostile

    #[test]
    fn repr_c_keeps_declaration_order() {
        assert_eq!(Ordered::SIZE, 12);
        assert_eq!(Ordered::FIELDS.iter().map(|f| f.offset).collect::<Vec<_>>(), [0, 4, 8]);
    }

    /// Наблюдение rustc 1.97 без -Zrandomize-layout: поля переставлены по
    /// убыванию выравнивания, и дыр нет. Гарантий нет — группа `observed`.
    #[test]
    fn observed_rust_repr_reorders() {
        assert_eq!(Unordered::SIZE, 8);
        assert_eq!(Unordered::FIELDS.iter().map(|f| f.offset).collect::<Vec<_>>(), [4, 0, 5]);
        assert_eq!(
            layout::<Unordered>().to_string(),
            "\
Unordered: 8 байт, выравнивание 4
  +0  b: u32 (4)
  +4  a: u8 (1)
  +5  c: u8 (1)
  +6  [дыра 2]
"
        );
    }
}
