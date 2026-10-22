//! Раздел 5.2: времена жизни в структурах, разбор без копирования.
//!
//! Записи смотрят в буфер приёма и не переживают его:
//!
//! ```compile_fail,E0597
//! use ch05::tlv::records;
//!
//! let first;
//! {
//!     let rx = vec![0x01, 0x02, 0xAA, 0xBB];
//!     first = records(&rx).next();
//! }
//! println!("{first:?}");
//! ```
//!
//! И не дают изменить буфер, пока живы:
//!
//! ```compile_fail,E0502
//! use ch05::tlv::records;
//!
//! let mut rx = vec![0x01, 0x02, 0xAA, 0xBB];
//! let first = records(&rx).next();
//! rx.fill(0); // DMA пишет следующий пакет
//! println!("{first:?}");
//! ```

// ANCHOR: tlv
/// Запись «тег, длина, значение» в пакете от датчика. Значение — срез
/// буфера приёма: байты не копируются.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Tlv<'a> {
    pub tag: u8,
    pub value: &'a [u8],
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TlvError {
    /// Пакет кончился посреди записи: столько байт не хватило.
    Truncated { at: usize, missing: usize },
}

/// Курсор по пакету. Время жизни `'a` — буфера, а не курсора: записи
/// переживают сам курсор.
pub struct Records<'a> {
    buf: &'a [u8],
    pos: usize,
}

pub fn records(buf: &[u8]) -> Records<'_> {
    Records { buf, pos: 0 }
}

impl<'a> Iterator for Records<'a> {
    type Item = Result<Tlv<'a>, TlvError>;

    fn next(&mut self) -> Option<Self::Item> {
        let at = self.pos;
        let rest = &self.buf[at..];
        if rest.is_empty() {
            return None;
        }
        let need = match rest {
            [_, len, ..] => 2 + *len as usize,
            _ => 2,
        };
        let Some(rec) = rest.get(..need) else {
            self.pos = self.buf.len(); // дальше разбирать нечего
            return Some(Err(TlvError::Truncated { at, missing: need - rest.len() }));
        };
        self.pos += need;
        Some(Ok(Tlv { tag: rec[0], value: &rec[2..] }))
    }
}

impl<'a> Records<'a> {
    /// Неразобранный хвост пакета — тоже срез исходного буфера.
    pub fn rest(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}
// ANCHOR_END: tlv

// ANCHOR: find
/// Значение первой записи с тегом. Результат смотрит в `buf`, а курсор
/// живёт только внутри функции.
pub fn find(buf: &[u8], tag: u8) -> Option<&[u8]> {
    records(buf).filter_map(Result::ok).find(|r| r.tag == tag).map(|r| r.value)
}
// ANCHOR_END: find

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn zero_copy() {
        // Температура (тег 1, 2 байта), имя датчика (тег 7, 6 байт), обрыв.
        let rx: Vec<u8> = [&[1u8, 2, 0x10, 0x09][..], &[7, 6], b"bme280", &[3, 4, 0xEE]].concat();
        let (t, name) = {
            let mut it = records(&rx);
            let t = it.next().unwrap().unwrap();
            let name = it.next().unwrap().unwrap();
            assert_eq!(it.rest(), [3, 4, 0xEE]);
            assert_eq!(it.next(), Some(Err(TlvError::Truncated { at: 12, missing: 3 })));
            assert_eq!(it.next(), None);
            (t, name)
        }; // курсора уже нет, записи живы
        assert_eq!(t, Tlv { tag: 1, value: &[0x10, 0x09] });
        assert_eq!(name.value, b"bme280");
        // Значение — тот же адрес в буфере, а не копия.
        assert_eq!(name.value.as_ptr(), rx[6..].as_ptr());
        assert_eq!(find(&rx, 7), Some(&b"bme280"[..]));
        assert_eq!(find(&rx, 9), None);
    }

    #[test]
    fn sizes() {
        use core::mem::size_of;
        // Тег, указатель и длина среза; время жизни места не занимает.
        assert_eq!(size_of::<Tlv<'_>>(), 3 * size_of::<usize>());
        // Ссылка не бывает нулевой: None помещается в нишу (раздел 6.4).
        assert_eq!(size_of::<Option<Tlv<'_>>>(), size_of::<Tlv<'_>>());
        assert_eq!(size_of::<Records<'_>>(), 3 * size_of::<usize>());
    }
}
