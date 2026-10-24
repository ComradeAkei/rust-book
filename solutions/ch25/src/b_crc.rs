//! Задача B: таблица CRC-32 в `const fn` и та же таблица синуса, что в
//! задаче 8.B строил `build.rs`, — теперь без сценария сборки.

// ANCHOR: crc32
/// Отражённый полином CRC-32 (IEEE 802.3, zlib, Ethernet).
pub const POLY: u32 = 0xEDB8_8320;

/// Таблица для отражённого CRC-32 с полиномом `poly`: `t[b]` — восемь
/// шагов сдвига вправо для младшего байта `b`.
pub const fn crc32_table(poly: u32) -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut b = 0;
    while b < 256 {
        let mut crc = b as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ poly } else { crc >> 1 };
            bit += 1;
        }
        t[b] = crc;
        b += 1;
    }
    t
}

/// Таблица считается при компиляции и лежит в `.rodata`: 1024 байта во
/// flash, кода её построения в прошивке нет.
pub static CRC32_TABLE: [u32; 256] = crc32_table(POLY);

/// CRC-32 по таблице. `const fn`: годится и при выполнении, и в константе.
pub const fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    let mut i = 0;
    while i < data.len() {
        crc = (crc >> 8) ^ CRC32_TABLE[((crc as u8) ^ data[i]) as usize];
        i += 1;
    }
    !crc
}

/// Контрольное значение алгоритма проверяется при сборке.
pub const CHECK: u32 = crc32(b"123456789");
const _: () = assert!(CHECK == 0xCBF4_3926, "CRC-32: контрольное значение не сошлось");
// ANCHOR_END: crc32

// ANCHOR: sine
/// sin(x) рядом Тейлора до x¹⁹ для 0 ≤ x ≤ π/2: ошибка меньше 1e-15.
/// В константах нет `f64::sin`: это вызов libm, а не арифметика.
const fn sin(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    let mut n = 1;
    while n < 10 {
        // Следующий член: −term · x² / ((2n)(2n+1)).
        term = -term * x2 / ((2 * n) as f64 * (2 * n + 1) as f64);
        sum += term;
        n += 1;
    }
    sum
}

/// Четверть периода в Q15, как в задаче 8.B: round(32767 · sin(i·π/512)).
pub const fn sine_table() -> [i16; 257] {
    let mut t = [0i16; 257];
    let mut i = 0;
    while i <= 256 {
        let x = i as f64 / 256.0 * core::f64::consts::FRAC_PI_2;
        // Значения неотрицательны: округление к ближайшему — +0,5 и
        // отбрасывание дробной части.
        t[i] = (32767.0 * sin(x) + 0.5) as i16;
        i += 1;
    }
    t
}

pub static SINE: [i16; 257] = sine_table();
// ANCHOR_END: sine
