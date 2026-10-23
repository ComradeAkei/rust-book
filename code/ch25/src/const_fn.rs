//! Раздел 25.2: `const fn`.
//!
//! Тело `const fn` выполняет интерпретатор компилятора, и вызывать в нём
//! можно только то, что интерпретатор умеет выполнить при компиляции.
//! Цикл `for` — это вызовы `IntoIterator::into_iter` и `Iterator::next`,
//! методы трейтов, а они в `const fn` пока недоступны:
//!
//! ```compile_fail,E0015
//! pub const fn sum(a: &[u32]) -> u32 {
//!     let mut s = 0;
//!     for x in a { // cannot use `for` loop on `&[u32]` in constant functions
//!         s += *x;
//!     }
//!     s
//! }
//! ```
//!
//! Оператор `?` — тоже трейт (`Try`), и тоже нестабилен в константах:
//!
//! ```compile_fail,E0658
//! pub const fn first(a: &[u8]) -> Option<u8> {
//!     let x = a.first()?; // `?` is not allowed on `Option<&u8>` in constant functions
//!     Some(*x)
//! }
//! ```
//!
//! Операторы над обобщённым типом — вызовы трейтов (`PartialEq::eq`):
//!
//! ```compile_fail,E0015
//! pub const fn same<T: PartialEq>(a: &T, b: &T) -> bool {
//!     a == b // cannot call non-const operator in constant functions
//! }
//! ```
//!
//! Плавающая точка в `const fn` разрешена (Rust 1.82), а синуса и корня
//! в константах нет: это вызовы libm, а не арифметика.
//!
//! ```compile_fail,E0015
//! pub const ROOT2: f64 = 2.0f64.sqrt(); // cannot call non-const method `f64::<impl f64>::sqrt` in constants
//! ```
//!
//! Паника при вычислении константы — ошибка компиляции E0080 с текстом
//! паники:
//!
//! ```compile_fail,E0080
//! use ch25::const_fn::uart_divisor;
//!
//! // 12 МГц / (16 · 921600) = 0,81: делитель меньше единицы.
//! pub const FAST: ch25::const_fn::Divisor = uart_divisor(12_000_000, 921_600);
//! ```
//!
//! Цикл, который не кончается, компилятор не ждёт вечно: после примерно
//! двух секунд интерпретации срабатывает запрещающая проверка
//! `long_running_const_eval`.
//!
//! ```compile_fail
//! const fn spin(n: u64) -> u64 {
//!     let mut i = 0;
//!     let mut x = 0u64;
//!     while i < n {
//!         x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
//!         i += 1;
//!     }
//!     x
//! }
//!
//! pub const BIG: u64 = spin(u64::MAX); // constant evaluation is taking a long time
//! ```

// ANCHOR: divisor
/// Делитель частоты UART PL011: целая часть `IBRD` и дробная `FBRD` в
/// шестьдесят четвёртых долях. BAUDDIV = UARTCLK / (16 · скорость).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divisor {
    pub ibrd: u16,
    pub fbrd: u8,
    /// Ошибка скорости в миллионных долях: со знаком, с округлением к нулю.
    pub error_ppm: i32,
}

/// Наибольшее допустимое расхождение скоростей двух сторон линии: 2 %.
pub const MAX_ERROR_PPM: i32 = 20_000;

/// Делитель для частоты `clock_hz` и скорости `baud`. Целочисленная
/// арифметика, `while` нет, но есть ветвления и паника — всё это
/// выполнимо при компиляции.
pub const fn uart_divisor(clock_hz: u32, baud: u32) -> Divisor {
    if baud == 0 {
        panic!("UART: скорость 0");
    }
    // BAUDDIV · 64 = clock · 64 / (16 · baud) = clock · 4 / baud,
    // с округлением к ближайшему.
    let div64 = (clock_hz as u64 * 4 + baud as u64 / 2) / baud as u64;
    let ibrd = div64 >> 6;
    let fbrd = (div64 & 63) as u8;
    if ibrd == 0 {
        panic!("UART: делитель меньше 1, скорость недостижима при этой частоте");
    }
    if ibrd > 0xFFFF || (ibrd == 0xFFFF && fbrd != 0) {
        panic!("UART: делитель больше 65535");
    }
    // Настоящая скорость: clock · 4 / div64. Ошибка в ppm, со знаком.
    let actual = clock_hz as i64 * 4 * 1_000_000 / div64 as i64;
    let error_ppm = ((actual - baud as i64 * 1_000_000) / baud as i64) as i32;
    if error_ppm > MAX_ERROR_PPM || error_ppm < -MAX_ERROR_PPM {
        panic!("UART: ошибка скорости больше 2 %");
    }
    Divisor { ibrd: ibrd as u16, fbrd, error_ppm }
}

/// Частота тактирования UART0 на плате и скорость линии.
pub const UART_CLOCK_HZ: u32 = 16_000_000;
pub const UART0: Divisor = uart_divisor(UART_CLOCK_HZ, 115_200);
// ANCHOR_END: divisor

// ANCHOR: mut-ref
/// `&mut` в `const fn` (Rust 1.83): вспомогательная функция заполняет
/// кусок массива, который строит другая `const fn`.
const fn fill(buf: &mut [u8], from: usize, to: usize, value: u8) {
    let mut i = from;
    while i < to {
        buf[i] = value;
        i += 1;
    }
}

/// Классы байтов для разбора команд: 1 — цифра, 2 — буква, 3 — пробел,
/// 0 — всё остальное. Таблица на 256 байт вместо цепочки сравнений.
pub const fn byte_classes() -> [u8; 256] {
    let mut t = [0u8; 256];
    fill(&mut t, b'0' as usize, b'9' as usize + 1, 1);
    fill(&mut t, b'A' as usize, b'Z' as usize + 1, 2);
    fill(&mut t, b'a' as usize, b'z' as usize + 1, 2);
    t[b' ' as usize] = 3;
    t[b'\t' as usize] = 3;
    t
}

pub static CLASSES: [u8; 256] = byte_classes();
// ANCHOR_END: mut-ref

// ANCHOR: float
/// Плавающая точка при компиляции: перевод в формат Q15 с округлением к
/// ближайшему. Арифметика и `as` в константах разрешены; `f64::round`
/// — функция `std`, а не `core`, поэтому округление записано через
/// сдвиг на половину и отбрасывание дробной части.
pub const fn q15(x: f64) -> i16 {
    if x >= 1.0 {
        return i16::MAX;
    }
    if x < -1.0 {
        return i16::MIN;
    }
    let y = x * 32768.0;
    (if y >= 0.0 { y + 0.5 } else { y - 0.5 }) as i16
}

/// Коэффициенты фильтра нижних частот, заданные числами с точкой.
pub const FILTER: [i16; 4] = [q15(0.125), q15(0.375), q15(0.375), q15(0.125)];
// ANCHOR_END: float

// ANCHOR: runtime
/// Та же функция при выполнении: частота известна только после
/// настройки тактирования. `const fn` — обычная функция, и здесь она
/// работает как обычная, а паника — обычная паника.
pub fn divisor_at_runtime(clock_hz: u32, baud: u32) -> Divisor {
    uart_divisor(clock_hz, baud)
}
// ANCHOR_END: runtime

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisors() {
        assert_eq!(UART0, Divisor { ibrd: 8, fbrd: 44, error_ppm: -799 });
        // 12 МГц: 6 + 33/64, та же ошибка −0,08 %.
        let d = divisor_at_runtime(12_000_000, 115_200);
        assert_eq!((d.ibrd, d.fbrd, d.error_ppm), (6, 33, -799));
        // 9600 бод при 12 МГц делятся без остатка.
        assert_eq!(divisor_at_runtime(12_000_000, 9600).error_ppm, 0);
    }

    #[test]
    #[should_panic(expected = "делитель меньше 1")]
    fn too_fast_panics_at_runtime() {
        divisor_at_runtime(12_000_000, 921_600);
    }

    #[test]
    fn tables() {
        assert_eq!(CLASSES[b'7' as usize], 1);
        assert_eq!(CLASSES[b'q' as usize], 2);
        assert_eq!(CLASSES[b' ' as usize], 3);
        assert_eq!(CLASSES[b'-' as usize], 0);
        assert_eq!(CLASSES.iter().filter(|&&c| c == 2).count(), 52);
        assert_eq!(FILTER, [4096, 12288, 12288, 4096]);
        assert_eq!((q15(1.0), q15(-1.0), q15(-0.5)), (32767, -32768, -16384));
    }
}
