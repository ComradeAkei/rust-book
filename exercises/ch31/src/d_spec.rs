//! Задача 31.D, часть 2: ошибка, которой не видит ни один инструмент
//! главы.
//!
//! Из описания протокола станции, раздел «Синхронизация времени»:
//!
//! > Команда SYNC несёт 6 байт: `epoch_s` — секунды Unix, `u32`, затем
//! > `offset_ms` — поправка в миллисекундах, `i16`. Все многобайтовые поля
//! > передаются в сетевом порядке, старшим байтом вперёд. Пример: полночь
//! > 1 января 2026 года UTC с поправкой −250 мс — `69 55 B9 00 FF 06`.
//!
//! Кодер и декодер ниже написаны одним человеком, тесты к ним — тоже, и
//! все тесты проходят. Найдите, что не так, допишите тест, который это
//! ловит, и исправьте.

// ANCHOR: sync
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TimeSync {
    pub epoch_s: u32,
    pub offset_ms: i16,
}

pub fn encode(s: TimeSync) -> [u8; 6] {
    let mut out = [0; 6];
    out[..4].copy_from_slice(&s.epoch_s.to_le_bytes());
    out[4..].copy_from_slice(&s.offset_ms.to_le_bytes());
    out
}

pub fn decode(bytes: &[u8]) -> Option<TimeSync> {
    let b: &[u8; 6] = bytes.try_into().ok()?;
    Some(TimeSync {
        epoch_s: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        offset_ms: i16::from_le_bytes([b[4], b[5]]),
    })
}
// ANCHOR_END: sync

// Не для Miri: proptest ищет файл регрессий через getcwd, а Miri в
// изоляции его не даёт, и сотни случаев шли бы минуты (раздел 31.6).
#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ANCHOR: tests
    proptest! {
        /// Круговой обход: любое значение переживает кодирование.
        #[test]
        fn round_trip(epoch_s: u32, offset_ms: i16) {
            let s = TimeSync { epoch_s, offset_ms };
            prop_assert_eq!(decode(&encode(s)), Some(s));
        }

        /// Любые 6 байт — какое-то значение, и оно кодируется в те же байты.
        #[test]
        fn every_six_bytes_decode(bytes: [u8; 6]) {
            let s = decode(&bytes).unwrap();
            prop_assert_eq!(encode(s), bytes);
        }
    }

    #[test]
    fn wrong_length() {
        assert_eq!(decode(&[0; 5]), None);
        assert_eq!(decode(&[0; 7]), None);
    }

    #[test]
    fn known_values() {
        let s = TimeSync { epoch_s: 1, offset_ms: -1 };
        assert_eq!(encode(s), [1, 0, 0, 0, 0xFF, 0xFF]);
    }
    // ANCHOR_END: tests
}
