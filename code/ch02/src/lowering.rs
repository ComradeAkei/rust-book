//! Раздел 2.5: во что компилируется `match`.
//!
//! Каждая функция показывает один способ, которым LLVM превращает `match`
//! в машинный код. Все они `#[inline(never)]` и `#[unsafe(no_mangle)]`,
//! чтобы `cargo asm` находил их по имени.

// ANCHOR: jump_table
/// Шаг маленькой виртуальной машины: восемь операций с разным кодом.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_exec(op: u8, a: u32, b: u32) -> u32 {
    match op {
        0 => a.wrapping_add(b),
        1 => a.wrapping_sub(b),
        2 => a.wrapping_mul(b),
        3 => a & b,
        4 => a | b,
        5 => a ^ b,
        6 => a.wrapping_shl(b),
        7 => a.wrapping_shr(b),
        _ => 0,
    }
}
// ANCHOR_END: jump_table

// ANCHOR: lookup_table
/// Код цифры для семисегментного индикатора, биты `gfedcba`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_segments(digit: u8) -> u8 {
    match digit {
        0 => 0x3F,
        1 => 0x06,
        2 => 0x5B,
        3 => 0x4F,
        4 => 0x66,
        5 => 0x6D,
        6 => 0x7D,
        7 => 0x07,
        8 => 0x7F,
        9 => 0x6F,
        _ => 0x00,
    }
}
// ANCHOR_END: lookup_table

// ANCHOR: bitmask
/// Пробельный символ ASCII.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}
// ANCHOR_END: bitmask

// ANCHOR: compare_tree
/// Служба по номеру порта: значения разбросаны, таблица была бы почти
/// пустой.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_service(port: u16) -> u8 {
    match port {
        21 => 1,   // FTP
        22 => 2,   // SSH
        23 => 3,   // Telnet
        80 => 4,   // HTTP
        443 => 5,  // HTTPS
        1883 => 6, // MQTT
        8080 => 7, // HTTP, альтернативный
        _ => 0,
    }
}
// ANCHOR_END: compare_tree

// ANCHOR: linear
/// Задержка АЦП в тактах для режима усреднения: значения ложатся на
/// прямую 10 + 3·m.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_adc_delay(mode: u8) -> u32 {
    match mode {
        0 => 10,
        1 => 13,
        2 => 16,
        3 => 19,
        4 => 22,
        _ => 0,
    }
}
// ANCHOR_END: linear

// ANCHOR: assume
/// То же, что `ch02_exec`, но вызывающий обещает, что `op < 8`. Проверка
/// границы перед таблицей переходов исчезает. Нарушенное обещание —
/// неопределённое поведение.
///
/// # Safety
///
/// `op` должен быть меньше 8.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn ch02_exec_assume(op: u8, a: u32, b: u32) -> u32 {
    // SAFETY: условие переложено на вызывающего, см. выше.
    unsafe { core::hint::assert_unchecked(op < 8) };
    match op {
        0 => a.wrapping_add(b),
        1 => a.wrapping_sub(b),
        2 => a.wrapping_mul(b),
        3 => a & b,
        4 => a | b,
        5 => a ^ b,
        6 => a.wrapping_shl(b),
        7 => a.wrapping_shr(b),
        _ => 0,
    }
}
// ANCHOR_END: assume

// ANCHOR: masked
/// Без `unsafe`: маска сама ограничивает код операции, и ветка `_`
/// недостижима. LLVM видит это и убирает проверку границы и панику.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_exec_masked(op: u8, a: u32, b: u32) -> u32 {
    match op & 7 {
        0 => a.wrapping_add(b),
        1 => a.wrapping_sub(b),
        2 => a.wrapping_mul(b),
        3 => a & b,
        4 => a | b,
        5 => a ^ b,
        6 => a.wrapping_shl(b),
        7 => a.wrapping_shr(b),
        _ => unreachable!(),
    }
}
// ANCHOR_END: masked

// ANCHOR: cold
/// Здесь была бы запись в журнал. `black_box` не даёт оптимизатору
/// выбросить вызов вместе со всей веткой.
#[inline(never)]
fn note_dropped(b: u8) {
    core::hint::black_box(b);
}

#[cold]
#[inline(never)]
fn note_dropped_cold(b: u8) {
    core::hint::black_box(b);
}

/// Сумма байтов данных. Байты от `0xF0` и выше — служебные, их
/// отбрасывают и отмечают.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_sum_data(xs: &[u8]) -> u32 {
    let mut s = 0u32;
    for &x in xs {
        if x >= 0xF0 {
            note_dropped(x);
            continue;
        }
        s += u32::from(x);
    }
    s
}

/// То же, но функция-обработчик помечена `#[cold]`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_sum_data_cold(xs: &[u8]) -> u32 {
    let mut s = 0u32;
    for &x in xs {
        if x >= 0xF0 {
            note_dropped_cold(x);
            continue;
        }
        s += u32::from(x);
    }
    s
}
// ANCHOR_END: cold

// ANCHOR: cold_path
/// Когда в редкой ветке нет функции, которую можно пометить `#[cold]`,
/// ветку отмечает `cold_path()`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch02_count_data(xs: &[u8], dropped: &mut u32) -> u32 {
    let mut s = 0u32;
    for &x in xs {
        if x >= 0xF0 {
            core::hint::cold_path();
            *dropped += 1;
            continue;
        }
        s += u32::from(x);
    }
    s
}
// ANCHOR_END: cold_path

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec() {
        assert_eq!(ch02_exec(0, 7, 5), 12);
        assert_eq!(ch02_exec(1, 7, 5), 2);
        assert_eq!(ch02_exec(6, 1, 33), 2);
        assert_eq!(ch02_exec(8, 7, 5), 0);
        for op in 0..8 {
            // SAFETY: op < 8.
            assert_eq!(unsafe { ch02_exec_assume(op, 0xDEAD, 3) }, ch02_exec(op, 0xDEAD, 3));
            assert_eq!(ch02_exec_masked(op | 0xF8, 0xDEAD, 3), ch02_exec(op, 0xDEAD, 3));
        }
    }

    #[test]
    fn segments() {
        assert_eq!(ch02_segments(8), 0x7F);
        assert_eq!(ch02_segments(10), 0);
    }

    #[test]
    fn space() {
        let spaces: Vec<u8> = (0..=255).filter(|&b| ch02_is_space(b)).collect();
        assert_eq!(spaces, b"\t\n\r ");
    }

    #[test]
    fn service() {
        assert_eq!(ch02_service(443), 5);
        assert_eq!(ch02_service(444), 0);
    }

    #[test]
    fn adc_delay() {
        let d: Vec<u32> = (0..6).map(ch02_adc_delay).collect();
        assert_eq!(d, [10, 13, 16, 19, 22, 0]);
    }

    #[test]
    fn cold_paths() {
        let xs = [1, 2, 0xF5, 3, 0xFF];
        assert_eq!(ch02_sum_data(&xs), 6);
        assert_eq!(ch02_sum_data_cold(&xs), 6);
        let mut dropped = 0;
        assert_eq!(ch02_count_data(&xs, &mut dropped), 6);
        assert_eq!(dropped, 2);
    }
}
