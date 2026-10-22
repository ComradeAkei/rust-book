//! Раздел 10.3: `impl Trait` в аргументах и в возвращаемом значении.
//!
//! `impl Trait` в аргументе — безымянный параметр типа, назвать его
//! turbofish нельзя:
//!
//! ```compile_fail,E0107
//! let c = ch10::impl_trait::crc8::<[u8; 2]>([1, 2]);
//! ```
//!
//! `impl Trait` в результате — один конкретный тип, который вызывающий не
//! видит. Две ветки с разными типами — ошибка:
//!
//! ```compile_fail,E0308
//! pub fn samples(fast: bool) -> impl Iterator<Item = u32> {
//!     if fast { (0..10).step_by(2) } else { (0..10).map(|x| x * 3) }
//! }
//! ```
//!
//! Вызывающий знает о скрытом типе только то, что написано после `impl`:
//!
//! ```compile_fail,E0308
//! let n: core::ops::Range<u32> = ch10::impl_trait::evens(10);
//! ```
//!
//! В редакции 2024 скрытый тип захватывает все параметры функции, включая
//! время жизни ссылки, даже если на деле её не использует. Итератор
//! индексов держит заём `v`:
//!
//! ```compile_fail,E0502
//! use ch10::impl_trait::indices;
//! let mut v = vec![1, 2, 3];
//! let it = indices(&v);
//! v.push(4);
//! assert_eq!(it.count(), 3);
//! ```
//!
//! `use<>` перечисляет захваченное явно, и пустой список заём отпускает:
//!
//! ```
//! use ch10::impl_trait::indices_free;
//! let mut v = vec![1, 2, 3];
//! let it = indices_free(&v);
//! v.push(4);
//! assert_eq!(it.count(), 3);
//! ```
//!
//! Обещать `use<>` можно, только если скрытый тип и правда не держит
//! ссылку:
//!
//! ```compile_fail,E0700
//! pub fn bytes(v: &[u8]) -> impl Iterator<Item = u8> + use<> {
//!     v.iter().copied()
//! }
//! ```

// ANCHOR: arg
/// CRC-8 (полином 0x07) по любому источнику байтов: массив, срез через
/// `copied()`, итератор, `Vec`. Запись короче, чем `<I: IntoIterator<Item
/// = u8>>`, а код тот же: своя копия на каждый тип аргумента.
pub fn crc8(data: impl IntoIterator<Item = u8>) -> u8 {
    data.into_iter().fold(0, |crc, b| {
        let mut c = crc ^ b;
        for _ in 0..8 {
            c = if c & 0x80 != 0 { (c << 1) ^ 0x07 } else { c << 1 };
        }
        c
    })
}
// ANCHOR_END: arg

// ANCHOR: ret
/// Чётные числа до `limit`. Настоящий тип — `StepBy<Range<u32>>`, но
/// вызывающий знает только, что это итератор `u32`.
pub fn evens(limit: u32) -> impl Iterator<Item = u32> + Clone {
    (0..limit).step_by(2)
}

/// Замыкание тоже можно вернуть только как `impl Fn`: у его типа нет имени.
pub fn scaler(num: i32, den: i32) -> impl Fn(i32) -> i32 {
    move |x| x * num / den
}
// ANCHOR_END: ret

// ANCHOR: capture
/// Индексы элементов `v`. Скрытый тип — `Range<usize>`, ссылок в нём нет,
/// но по правилам редакции 2024 он захватывает время жизни `v`.
pub fn indices(v: &[u32]) -> impl Iterator<Item = usize> {
    0..v.len()
}

/// То же с явным пустым списком захвата: заём `v` кончается на выходе.
pub fn indices_free(v: &[u32]) -> impl Iterator<Item = usize> + use<> {
    0..v.len()
}
// ANCHOR_END: capture

// ANCHOR: rpitit
/// `impl Trait` в методе трейта: каждая реализация возвращает свой тип.
pub trait Source {
    fn samples(&self) -> impl Iterator<Item = i16>;
}

/// Буфер АЦП отдаёт свои отсчёты.
pub struct AdcBuf(pub [i16; 4]);

impl Source for AdcBuf {
    fn samples(&self) -> impl Iterator<Item = i16> {
        self.0.iter().copied()
    }
}

/// Генератор пилы ничего не хранит и считает отсчёты на ходу.
pub struct Saw {
    pub step: i16,
}

impl Source for Saw {
    fn samples(&self) -> impl Iterator<Item = i16> {
        let step = self.step;
        (0..4).map(move |i| i * step)
    }
}

/// Обобщённая функция над любым `Source`.
pub fn peak<S: Source>(s: &S) -> Option<i16> {
    s.samples().max()
}
// ANCHOR_END: rpitit

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn argument_position() {
        let data = *b"123456789";
        assert_eq!(crc8(data), 0xf4);
        assert_eq!(crc8(data.iter().copied()), 0xf4);
        assert_eq!(crc8(Vec::from(data)), 0xf4);
        assert_eq!(crc8("123456789".bytes()), 0xf4);
    }

    #[test]
    fn return_position() {
        let e = evens(7);
        assert_eq!(e.clone().collect::<Vec<_>>(), [0, 2, 4, 6]);
        assert_eq!(e.sum::<u32>(), 12);
        // Скрытый тип известен компилятору, и type_name его показывает.
        assert!(
            core::any::type_name_of_val(&evens(7)).contains("StepBy<core::ops::range::Range<u32>>")
        );
        let to_mv = scaler(3300, 4095);
        assert_eq!(to_mv(4095), 3300);
        assert_eq!(size_of_val(&to_mv), 8);
        let mut v = vec![1, 2, 3];
        let it = indices_free(&v);
        v.push(4);
        assert_eq!(it.count(), 3);
        assert_eq!(indices(&v).count(), 4);
    }

    #[test]
    fn rpitit() {
        assert_eq!(peak(&AdcBuf([-5, 700, 12, 3])), Some(700));
        assert_eq!(peak(&Saw { step: 100 }), Some(300));
    }
}
