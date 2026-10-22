//! Раздел 2.1: всё есть выражение.
//!
//! Точка с запятой превращает выражение в инструкцию, и значение блока
//! становится `()`:
//!
//! ```compile_fail,E0308
//! fn twice(x: u32) -> u32 {
//!     x * 2;
//! }
//! ```

// ANCHOR: block
/// Делитель частоты для UART с округлением к ближайшему. Промежуточные
/// значения живут только внутри блока, наружу выходит результат. Регистр
/// делителя 16-битный.
pub fn uart_divisor(clock_hz: u32, baud: u32) -> u16 {
    let div = {
        let x16 = clock_hz / baud; // частота в шестнадцатых долях бита
        let rounded = (x16 + 8) / 16;
        if rounded == 0 { 1 } else { rounded }
    };
    u16::try_from(div).unwrap_or(u16::MAX)
}
// ANCHOR_END: block

// ANCHOR: if_expr
/// Уровень заряда по напряжению батареи в милливольтах. `if` — выражение,
/// и оно же значение функции: тернарный оператор не нужен.
pub fn battery_level(mv: u32) -> u8 {
    if mv >= 4100 {
        100
    } else if mv <= 3300 {
        0
    } else {
        ((mv - 3300) / 8) as u8
    }
}
// ANCHOR_END: if_expr

// ANCHOR: never
/// Десятичное число с разделителями `_`. Ветки с `continue` и `return`
/// имеют тип `!` и подходят к любому типу, поэтому `match` целиком —
/// `u32`.
pub fn parse_u32(s: &[u8]) -> Option<u32> {
    let mut acc: u32 = 0;
    for &b in s {
        let digit = match b {
            b'0'..=b'9' => u32::from(b - b'0'),
            b'_' => continue,
            _ => return None,
        };
        acc = acc.checked_mul(10)?.checked_add(digit)?;
    }
    Some(acc)
}
// ANCHOR_END: never

// ANCHOR: labeled_block
/// Контрольная сумма заголовка: первые два байта — маркер и длина. Блок с
/// меткой выходит со значением из середины, как `goto` на общий выход.
pub fn header_sum(frame: &[u8]) -> Option<u8> {
    'check: {
        if frame.len() < 2 {
            break 'check None;
        }
        if frame[0] != 0x7E {
            break 'check None;
        }
        Some(frame[0].wrapping_add(frame[1]))
    }
}
// ANCHOR_END: labeled_block

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisor() {
        // 16 МГц и 115200 бод: 8,68 → 9.
        assert_eq!(uart_divisor(16_000_000, 115_200), 9);
        assert_eq!(uart_divisor(16_000_000, 9600), 104);
        assert_eq!(uart_divisor(1000, 115_200), 1);
    }

    #[test]
    fn battery() {
        assert_eq!(battery_level(4200), 100);
        assert_eq!(battery_level(3000), 0);
        assert_eq!(battery_level(3700), 50);
    }

    #[test]
    fn parse() {
        assert_eq!(parse_u32(b"1_000_000"), Some(1_000_000));
        assert_eq!(parse_u32(b"4294967295"), Some(u32::MAX));
        assert_eq!(parse_u32(b"4294967296"), None);
        assert_eq!(parse_u32(b"12a"), None);
        assert_eq!(parse_u32(b""), Some(0));
    }

    #[test]
    fn labeled() {
        assert_eq!(header_sum(&[0x7E, 3, 1, 2, 3]), Some(0x81));
        assert_eq!(header_sum(&[0x7E]), None);
        assert_eq!(header_sum(&[0x00, 3]), None);
    }
}
