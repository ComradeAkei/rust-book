//! Раздел 13.2: адаптеры и ленивость. Каждый пример — маленькая задача
//! обработки отсчётов или байтов протокола.

use core::cell::Cell;

#[cfg(feature = "alloc")]
use alloc::string::String;

// ANCHOR: lazy
/// Сколько раз вызвано замыкание в `map`: после построения цепочки, после
/// первого `next` и после того, как цепочку дочитали.
pub fn lazy_calls() -> ([usize; 3], Option<i32>, i32) {
    let calls = Cell::new(0);
    let data = [3, 8, 1, 9, 4, 7];
    let tens = data.iter().map(|x| {
        calls.set(calls.get() + 1);
        x * 10
    });
    let built = calls.get(); //                  0: цепочка ещё не работала
    let mut big = tens.filter(|&x| x > 50);
    let first = big.next(); //                   Some(80)
    let after_first = calls.get(); //            2: отсчёты 3 и 8
    let rest = big.sum(); //                     90 + 70
    ([built, after_first, calls.get()], first, rest)
}
// ANCHOR_END: lazy

// ANCHOR: mean
/// Среднее напряжение в милливольтах по сырым отсчётам 12-битного АЦП с
/// опорой 3,3 В. Отсчёты 0 и 4095 — насыщение, их не считают.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_mean_mv(raw: &[u16]) -> Option<u32> {
    let (sum, n) = raw
        .iter()
        .filter(|&&r| r != 0 && r != 4095)
        .map(|&r| u32::from(r) * 3300 / 4095)
        .fold((0u32, 0u32), |(s, n), mv| (s + mv, n + 1));
    sum.checked_div(n)
}
// ANCHOR_END: mean

// ANCHOR: zip
/// Номер первого байта, в котором принятый кадр расходится с ожидаемым.
/// `zip` останавливается на конце более короткого.
pub fn first_mismatch(got: &[u8], want: &[u8]) -> Option<usize> {
    got.iter().zip(want).position(|(a, b)| a != b)
}

/// Номера всех несовпавших байтов в буфере `out`; возвращает, сколько их.
pub fn mismatches(got: &[u8], want: &[u8], out: &mut [usize]) -> usize {
    let bad = got.iter().zip(want).enumerate().filter(|(_, (a, b))| a != b);
    out.iter_mut().zip(bad).map(|(o, (i, _))| *o = i).count()
}
// ANCHOR_END: zip

// ANCHOR: chain
/// CRC-16/CCITT-FALSE по любой последовательности байт.
pub fn crc16<'a>(bytes: impl IntoIterator<Item = &'a u8>) -> u16 {
    bytes.into_iter().fold(0xFFFF, |crc, &b| {
        let mut c = crc ^ (u16::from(b) << 8);
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
        c
    })
}

/// CRC кадра считается по заголовку без поля CRC и по нагрузке, которые
/// лежат в разных местах: `chain` склеивает их без копирования.
pub fn frame_crc(head: &[u8; 6], payload: &[u8]) -> u16 {
    crc16(head.iter().chain(payload))
}
// ANCHOR_END: chain

// ANCHOR: flat_map
#[cfg(feature = "alloc")]
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Шестнадцатеричный дамп: каждый байт превращается в две цифры, и
/// `flat_map` разворачивает пары в одну последовательность.
#[cfg(feature = "alloc")]
pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|&b| [HEX[usize::from(b >> 4)], HEX[usize::from(b & 15)]])
        .map(char::from)
        .collect()
}
// ANCHOR_END: flat_map

// ANCHOR: scan
/// Датчик шлёт первое значение целиком, а дальше — разности. `scan` несёт
/// текущее значение как состояние и обрывает поток на переполнении.
pub fn undelta(first: i16, deltas: &[i16]) -> impl Iterator<Item = i16> {
    let tail = deltas.iter().scan(first, |acc, &d| {
        *acc = acc.checked_add(d)?;
        Some(*acc)
    });
    core::iter::once(first).chain(tail)
}
// ANCHOR_END: scan

// ANCHOR: while
/// Строка UART: преамбула из байтов 0x55, затем текст до нуля.
/// `skip_while` пропускает преамбулу, `take_while` останавливается на нуле.
pub fn payload(line: &[u8]) -> impl Iterator<Item = &u8> {
    line.iter().skip_while(|&&b| b == 0x55).take_while(|&&b| b != 0)
}

/// Число из десятичных цифр в начале буфера: `map_while` и отображает, и
/// останавливается — на первом байте, который не цифра.
pub fn leading_number(s: &[u8]) -> u32 {
    s.iter().map_while(|&b| char::from(b).to_digit(10)).fold(0, |n, d| n * 10 + d)
}
// ANCHOR_END: while

// ANCHOR: peekable
/// Кодирование длинами серий: `[1, 1, 1, 0, 0, 7]` → `(1, 3), (0, 2),
/// (7, 1)`. `peekable` позволяет заглянуть в следующий байт, не забирая
/// его, а `next_if_eq` забирает только равный.
pub fn rle(bytes: &[u8], out: &mut [(u8, usize)]) -> usize {
    let mut it = bytes.iter().copied().peekable();
    let mut n = 0;
    while let (Some(b), Some(slot)) = (it.next(), out.get_mut(n)) {
        let mut run = 1;
        while it.next_if_eq(&b).is_some() {
            run += 1;
        }
        *slot = (b, run);
        n += 1;
    }
    n
}
// ANCHOR_END: peekable

// ANCHOR: windows
/// Скользящее среднее по четырём отсчётам: окна перекрываются, каждое —
/// срез из четырёх элементов исходного буфера.
pub fn moving_avg4(x: &[i16]) -> impl Iterator<Item = i16> {
    x.windows(4).map(|w| (w.iter().map(|&s| i32::from(s)).sum::<i32>() / 4) as i16)
}

/// Слова little-endian из байтов: `chunks_exact(2)` даёт куски ровно по
/// два байта, а неполный хвост — отдельно, через `remainder`.
pub fn words_le(bytes: &[u8]) -> (impl Iterator<Item = u16>, &[u8]) {
    let chunks = bytes.chunks_exact(2);
    let tail = chunks.remainder();
    (chunks.map(|c| u16::from_le_bytes([c[0], c[1]])), tail)
}
// ANCHOR_END: windows

// ANCHOR: step_rev
/// Прореживание в четыре раза и обратный порядок: последний отсчёт
/// каждой четвёрки, начиная с конца буфера.
pub fn decimate_rev(x: &[i16]) -> impl Iterator<Item = i16> {
    x.iter().rev().step_by(4).copied()
}

/// Испытательный шаблон для линии: 0xAA, 0x55, 0xAA, … ровно `n` байт.
pub fn test_pattern(n: usize) -> impl Iterator<Item = u8> {
    [0xAA, 0x55].into_iter().cycle().take(n)
}
// ANCHOR_END: step_rev

// ANCHOR: type_name
/// Тип цепочки — вложенные обобщённые структуры, по одной на адаптер, с
/// типами замыканий внутри.
#[cfg(feature = "alloc")]
pub fn chain_type() -> (&'static str, usize) {
    let data = [3i32, 8, 1];
    let it = data.iter().map(|x| x * 2).filter(|x| *x > 2);
    (core::any::type_name_of_val(&it), size_of_val(&it))
}
// ANCHOR_END: type_name

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lazy() {
        assert_eq!(lazy_calls(), ([0, 2, 6], Some(80), 160));
    }

    #[test]
    #[allow(clippy::needless_range_loop)] // цикл по индексам — эталон для сравнения
    fn mean() {
        assert_eq!(ch13_mean_mv(&[0, 4095, 4095]), None);
        assert_eq!(ch13_mean_mv(&[]), None);
        // 2048 → 1650 мВ, 1024 → 825 мВ, 0 и 4095 отброшены.
        assert_eq!(ch13_mean_mv(&[2048, 0, 1024, 4095]), Some((1650 + 825) / 2));
        // Тот же ответ циклом по индексам.
        let raw: Vec<u16> = (0..1000u32).map(|i| (i * 37 % 4096) as u16).collect();
        let (mut s, mut n) = (0u32, 0u32);
        for i in 0..raw.len() {
            if raw[i] != 0 && raw[i] != 4095 {
                s += u32::from(raw[i]) * 3300 / 4095;
                n += 1;
            }
        }
        assert_eq!(ch13_mean_mv(&raw), Some(s / n));
    }

    #[test]
    fn zip() {
        assert_eq!(first_mismatch(b"\xa5\x01\x02", b"\xa5\x01\x03"), Some(2));
        assert_eq!(first_mismatch(b"\xa5\x01", b"\xa5\x01\x03"), None);
        let mut out = [0; 4];
        let n = mismatches(b"abcdef", b"abXdYf", &mut out);
        assert_eq!(&out[..n], [2, 4]);
        let mut one = [0; 1];
        assert_eq!(mismatches(b"abcdef", b"XbXdYf", &mut one), 1);
        assert_eq!(one, [0]);
    }

    #[test]
    fn chain() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
        // Кадр MEAS из тестов проекта: CRC 0x73e5 в байтах 6 и 7.
        let head = [0xa5, 0x01, 0x01, 0x05, 0x0a, 0x00];
        let payload = [0x07, 0x00, 0xe8, 0x03, 0x00, 0x00, 0xd6, 0xff, 0xff, 0xff];
        assert_eq!(frame_crc(&head, &payload), 0x73e5);
    }

    #[test]
    fn flat_map() {
        assert_eq!(hex(&[0xa5, 0x01, 0xff]), "a501ff");
        assert_eq!(hex(&[]), "");
    }

    #[test]
    fn scan() {
        assert_eq!(undelta(100, &[2, -1, 5]).collect::<Vec<_>>(), [100, 102, 101, 106]);
        assert_eq!(undelta(i16::MAX - 1, &[1, 1, -5]).collect::<Vec<_>>(), [32766, 32767]);
    }

    #[test]
    fn take_skip_while() {
        let line = b"\x55\x55\x55T=21\0junk";
        assert_eq!(payload(line).copied().collect::<Vec<_>>(), b"T=21");
        assert_eq!(leading_number(b"1234,56"), 1234);
        assert_eq!(leading_number(b"x1"), 0);
    }

    #[test]
    fn peekable() {
        let mut out = [(0, 0); 4];
        let n = rle(&[1, 1, 1, 0, 0, 7], &mut out);
        assert_eq!(&out[..n], [(1, 3), (0, 2), (7, 1)]);
        assert_eq!(rle(&[], &mut out), 0);
        let mut two = [(0, 0); 2];
        assert_eq!(rle(&[1, 2, 3], &mut two), 2);
        assert_eq!(two, [(1, 1), (2, 1)]);
    }

    #[test]
    fn windows_chunks() {
        let x = [4, 8, 12, 16, 20];
        assert_eq!(moving_avg4(&x).collect::<Vec<_>>(), [10, 14]);
        assert_eq!(moving_avg4(&x[..3]).count(), 0);
        let (words, tail) = words_le(&[0x34, 0x12, 0xcd, 0xab, 0xee]);
        assert_eq!(words.collect::<Vec<_>>(), [0x1234, 0xabcd]);
        assert_eq!(tail, [0xee]);
    }

    #[test]
    fn step_rev() {
        let x: Vec<i16> = (0..10).collect();
        assert_eq!(decimate_rev(&x).collect::<Vec<_>>(), [9, 5, 1]);
        assert_eq!(test_pattern(5).collect::<Vec<_>>(), [0xAA, 0x55, 0xAA, 0x55, 0xAA]);
    }

    /// Имя типа и размеры адаптеров у rustc 1.97: ни то ни другое не
    /// гарантировано.
    #[test]
    fn observed_adapter_types() {
        let (name, size) = chain_type();
        assert!(
            name.starts_with(
                "core::iter::adapters::filter::Filter<core::iter::adapters::map::Map<\
                 core::slice::iter::Iter<'_, i32>, ch13::adapters::chain_type::{{closure}}>"
            ),
            "{name}"
        );
        let w = size_of::<usize>();
        assert_eq!(size, 2 * w);
        let d = [1i32, 2, 3];
        let k = 3;
        assert_eq!(size_of_val(&d.iter().map(|x| x * 2)), 2 * w);
        assert_eq!(size_of_val(&d.iter().map(|x| x * k)), 3 * w);
        assert_eq!(size_of_val(&d.iter().enumerate()), 3 * w);
        assert_eq!(size_of_val(&d.iter().chain(&d)), 4 * w);
        assert_eq!(size_of_val(&d.iter().peekable()), 4 * w);
        assert_eq!(size_of_val(&d.iter().zip(&d)), 6 * w);
        assert_eq!(size_of_val(&d.chunks_exact(2)), 5 * w);
    }
}
