//! Раздел 25.5: таблицы и разбор литералов при компиляции.
//!
//! Ошибка в литерале `hex!` — ошибка сборки, а не паника при запуске:
//!
//! ```compile_fail,E0080
//! const BAD: [u8; 2] = ch25::hex!("A5 0G"); // hex!: не шестнадцатеричная цифра
//! ```
//!
//! ```compile_fail,E0080
//! const ODD: [u8; 2] = ch25::hex!("A5 0"); // hex!: нечётное число цифр
//! ```
//!
//! Длина результата — часть типа, и её проверяет проверка типов:
//!
//! ```compile_fail,E0308
//! const W: [u8; 3] = ch25::hex!("A5 01"); // expected an array with a size of 3, found one with a size of 2
//! ```

// ANCHOR: recip
/// Обратные значения для деления без деления: `RECIP[d] = ⌈2²⁰ / d⌉`.
///
/// У Cortex-M0 нет инструкции деления, и `x / d` — вызов
/// `__aeabi_uidiv`, программное деление в несколько десятков тактов.
/// Для 12-битных отсчётов АЦП (`x < 4096`) и делителей до 255 частное
/// равно `(x · RECIP[d]) >> 20` точно: произведение меньше 2³², хватает
/// одной инструкции `muls`.
pub const RECIP_SHIFT: u32 = 20;

pub const fn reciprocals() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut d = 1;
    while d < 256 {
        t[d] = (1u32 << RECIP_SHIFT).div_ceil(d as u32);
        d += 1;
    }
    t
}

pub static RECIP: [u32; 256] = reciprocals();

/// `x / d` для 12-битного `x` и делителя 1..=255. Деления нет.
#[inline]
pub fn div_small(x: u32, d: u8) -> u32 {
    debug_assert!(x < 4096 && d > 0);
    (x * RECIP[d as usize]) >> RECIP_SHIFT
}
// ANCHOR_END: recip

// ANCHOR: scale
/// Отсчёт АЦП в единицы датчика: калибровочный делитель из таблицы
/// датчиков. На Cortex-M0 — вызов `__aeabi_uidiv`.
#[unsafe(no_mangle)]
pub extern "C" fn ch25_scale_div(code: u32, k: core::num::NonZeroU8) -> u32 {
    code / k.get() as u32
}

/// То же через таблицу: загрузка из `RECIP`, `muls` и сдвиг.
#[unsafe(no_mangle)]
pub extern "C" fn ch25_scale_recip(code: u32, k: core::num::NonZeroU8) -> u32 {
    div_small(code, k.get())
}
// ANCHOR_END: scale

// ANCHOR: const-div
/// Деление на константу. На Cortex-M3 компилятор заменяет его умножением
/// на «магическое» число с 64-битным результатом (`umull`); у Cortex-M0
/// такого умножения нет, и остаётся вызов `__aeabi_uidiv`.
#[unsafe(no_mangle)]
pub extern "C" fn ch25_scale_div7(code: u32) -> u32 {
    code / 7
}
// ANCHOR_END: const-div

// ANCHOR: hex
/// Значение шестнадцатеричной цифры; паника — ошибка компиляции, если
/// функцию вызвали в константе.
const fn hex_digit(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("hex!: не шестнадцатеричная цифра"),
    }
}

/// Сколько байт в строке вида "A5 01 ff": пробелы между байтами
/// разрешены, цифр должно быть чётное число.
pub const fn hex_len(s: &str) -> usize {
    let b = s.as_bytes();
    let mut digits = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] != b' ' {
            digits += 1;
        }
        i += 1;
    }
    if digits % 2 != 0 {
        panic!("hex!: нечётное число цифр");
    }
    digits / 2
}

/// Байты строки. Длину `N` даёт `hex_len` той же строки.
pub const fn hex_bytes<const N: usize>(s: &str) -> [u8; N] {
    let b = s.as_bytes();
    let mut out = [0u8; N];
    let (mut i, mut n) = (0, 0);
    while i < b.len() {
        if b[i] == b' ' {
            i += 1;
            continue;
        }
        if i + 1 >= b.len() || b[i + 1] == b' ' {
            panic!("hex!: байт из одной цифры");
        }
        out[n] = hex_digit(b[i]) << 4 | hex_digit(b[i + 1]);
        n += 1;
        i += 2;
    }
    out
}

/// Байты из шестнадцатеричной строки при компиляции: тип результата —
/// `[u8; N]`, где `N` посчитан по той же строке.
#[macro_export]
macro_rules! hex {
    ($s:literal) => {{
        const S: &str = $s;
        const N: usize = $crate::tables::hex_len(S);
        const BYTES: [u8; N] = $crate::tables::hex_bytes::<N>(S);
        BYTES
    }};
}
// ANCHOR_END: hex

// ANCHOR: vector
/// Тестовый кадр протокола станции (раздел 6.11) так, как его печатает
/// анализатор шины: измерение, номер 7, нагрузка 12 34 и CRC 0x9BD2.
pub const MEASUREMENT_FRAME: [u8; 10] = hex!("A5 01 01 07 02 00 D2 9B 12 34");
// ANCHOR_END: vector

#[cfg(test)]
mod tests {
    use super::*;

    /// Гарантия задачи: частное точное на всех 12-битных x и делителях
    /// 1..=255 — проверено перебором. Под Miri — каждое 61-е x и
    /// наибольшее: полный перебор там шёл бы минуты.
    #[test]
    fn reciprocal_division_is_exact() {
        let step = if cfg!(miri) { 61 } else { 1 };
        for d in 1..=255u8 {
            for x in (0..4096u32).step_by(step).chain([4095]) {
                assert_eq!(div_small(x, d), x / d as u32, "{x} / {d}");
            }
        }
        assert_eq!(RECIP[1], 1 << 20);
        assert_eq!(RECIP[3], 349_526);
    }

    /// А при 13-битных x точность теряется даже без переполнения, в
    /// 64-битной арифметике: граница из условия — не перестраховка.
    #[test]
    #[cfg_attr(miri, ignore = "перебор шестисот тысяч пар; unsafe здесь нет")]
    fn reciprocal_breaks_beyond_twelve_bits() {
        let wrong = |x: u64, d: u64| (x * RECIP[d as usize] as u64) >> RECIP_SHIFT != x / d;
        let bad = (1..=255u64).flat_map(|d| (4096..8192u64).map(move |x| (x, d)));
        let first = bad.clone().find(|&(x, d)| wrong(x, d));
        // Первый сбой по возрастанию делителя: 7445 / 146 = 50,99, а по
        // таблице выходит 51.
        assert_eq!(first, Some((7445, 146)));
        // 8191 · RECIP[255] ещё помещается в u32, 8191 · RECIP[1] — уже нет.
        assert!(8191u64 * RECIP[1] as u64 > u32::MAX as u64);
    }

    #[test]
    fn hex_literals() {
        assert_eq!(hex!("A5 01 ff"), [0xA5, 0x01, 0xFF]);
        assert_eq!(hex!("deadBEEF"), [0xDE, 0xAD, 0xBE, 0xEF]);
        let empty: [u8; 0] = hex!("");
        assert_eq!(empty, []);
        assert_eq!(MEASUREMENT_FRAME[0], 0xA5);
        assert_eq!(MEASUREMENT_FRAME.len(), 10);
        // При выполнении те же функции работают как обычные.
        assert_eq!(hex_bytes::<2>("12 34"), [0x12, 0x34]);
    }

    #[test]
    #[should_panic(expected = "байт из одной цифры")]
    fn split_byte_panics_at_runtime() {
        hex_len("A 5");
        hex_bytes::<1>("A 5");
    }
}
