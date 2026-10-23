//! Раздел 15.8: коллекции без кучи. `heapless::Vec`, `String`, `Deque` и
//! `FnvIndexMap` держат элементы в себе, ёмкость — параметр типа. Место
//! кончается — вставка возвращает `Err`, а не просит память: на
//! микроконтроллере это обычный исход, а не авария.

use heapless::index_map::FnvIndexMap;
use heapless::{Deque, String, Vec};

// ANCHOR: last_values
/// Последнее значение каждого датчика, не больше 8 датчиков. Девятый
/// датчик не помещается — его номер возвращается вызывающему.
pub fn last_values(readings: &[(u16, i32)]) -> Result<FnvIndexMap<u16, i32, 8>, u16> {
    let mut m = FnvIndexMap::new();
    for &(sensor, value) in readings {
        m.insert(sensor, value).map_err(|(sensor, _)| sensor)?;
    }
    Ok(m)
}
// ANCHOR_END: last_values

// ANCHOR: line
/// Строка отчёта в буфере на стеке: `write!` в `heapless::String` вместо
/// `snprintf`. Не помещается — `Err`, а не обрезанная строка.
pub fn line(sensor: u16, value: i32) -> Result<String<16>, core::fmt::Error> {
    use core::fmt::Write;
    let mut s = String::new();
    write!(s, "s{sensor}={value}")?;
    Ok(s)
}
// ANCHOR_END: line

// ANCHOR: fifo
/// Очередь кадров на отправку: `Deque` — кольцевой буфер фиксированной
/// ёмкости. Полная очередь отдаёт кадр обратно.
pub fn enqueue(q: &mut Deque<[u8; 18], 4>, frame: [u8; 18]) -> Result<(), [u8; 18]> {
    q.push_back(frame)
}
// ANCHOR_END: fifo

// ANCHOR: sizes
/// Размеры коллекций heapless: элементы внутри, рядом длина. Тип длины —
/// третий параметр `Vec`, по умолчанию `usize`.
pub const fn sizes() -> [usize; 5] {
    [
        size_of::<Vec<u8, 16>>(),
        size_of::<Vec<u8, 16, u8>>(),
        size_of::<String<16>>(),
        size_of::<Deque<[u8; 18], 4>>(),
        size_of::<FnvIndexMap<u16, i32, 8>>(),
    ]
}
// ANCHOR_END: sizes

/// Буфер приёма: байты копятся, пока не наберётся кадр или не кончится
/// место.
pub fn collect_bytes(input: &[u8]) -> (Vec<u8, 8>, usize) {
    let mut v = Vec::new();
    let mut dropped = 0;
    for &b in input {
        if v.push(b).is_err() {
            dropped += 1;
        }
    }
    (v, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_reports_ninth_sensor() {
        let ok: [(u16, i32); 3] = [(1, 10), (2, 20), (1, 11)];
        let m = last_values(&ok).unwrap();
        assert_eq!((m.len(), m[&1], m[&2]), (2, 11, 20));
        let many: [(u16, i32); 9] = core::array::from_fn(|i| (i as u16 + 1, 0));
        assert_eq!(last_values(&many).unwrap_err(), 9);
    }

    #[test]
    fn line_fits_or_errs() {
        assert_eq!(line(7, -42).unwrap(), "s7=-42");
        assert!(line(65535, i32::MIN).is_err()); // 18 байт
    }

    #[test]
    fn fifo_full() {
        let mut q = Deque::new();
        for i in 0..4 {
            assert!(enqueue(&mut q, [i; 18]).is_ok());
        }
        assert_eq!(enqueue(&mut q, [9; 18]), Err([9; 18]));
        assert_eq!(q.pop_front(), Some([0; 18]));
    }

    #[test]
    fn bytes_dropped_when_full() {
        let (v, dropped) = collect_bytes(&[1; 11]);
        assert_eq!((v.len(), dropped), (8, 3));
    }
}
