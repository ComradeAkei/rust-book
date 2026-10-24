//! Задача D: буферы драйверов против бюджета ОЗУ — проверка при сборке.
//!
//! Бюджет приходит из `build.rs` (объём ОЗУ из firmware/memory.x минус
//! запас под стек), размеры буферов — из типов через `size_of`. Если
//! сумма не помещается, сборка останавливается с понятным текстом, а
//! вторая проверка называет число лишних байт.

use core::mem::size_of;

// ANCHOR: parse
/// Десятичное число из строки окружения сборки. `const fn`, потому что
/// результат нужен константе `BUDGET`.
pub const fn parse_budget(s: &str) -> usize {
    let b = s.as_bytes();
    assert!(!b.is_empty(), "CH25_RAM_BUDGET: пустая строка");
    let mut n = 0usize;
    let mut i = 0;
    while i < b.len() {
        assert!(b[i].is_ascii_digit(), "CH25_RAM_BUDGET: не десятичное число");
        n = n * 10 + (b[i] - b'0') as usize;
        i += 1;
    }
    n
}

/// Сколько байт ОЗУ отдано буферам драйверов.
pub const BUDGET: usize = parse_budget(env!("CH25_RAM_BUDGET"));
// ANCHOR_END: parse

// ANCHOR: drivers
/// Буферы UART: передача и приём.
pub struct UartBuffers {
    pub tx: [u8; 512],
    pub rx: [u8; 256],
}

/// Отсчёты АЦП за один период усреднения.
pub type AdcSamples = [u16; 1024];

/// Кольцевой журнал событий.
pub type EventLog = [u8; 4096];

/// Очередь кадров радиомодуля: 16 кадров по 64 байта.
pub type RadioQueue = [[u8; 64]; 16];

/// Драйвер и сколько ОЗУ занимают его буферы.
pub struct Driver {
    pub name: &'static str,
    pub ram: usize,
}

/// Реестр драйверов станции. Размеры — из типов: поменяли буфер —
/// поменялась и проверка.
pub const DRIVERS: [Driver; 4] = [
    Driver { name: "uart", ram: size_of::<UartBuffers>() },
    Driver { name: "adc", ram: size_of::<AdcSamples>() },
    Driver { name: "log", ram: size_of::<EventLog>() },
    Driver { name: "radio", ram: size_of::<RadioQueue>() },
];
// ANCHOR_END: drivers

// ANCHOR: check
pub const fn total(drivers: &[Driver]) -> usize {
    let mut sum = 0;
    let mut i = 0;
    while i < drivers.len() {
        sum += drivers[i].ram;
        i += 1;
    }
    sum
}

pub const USED: usize = total(&DRIVERS);

// Текст — литерал: число в сообщение не подставить (раздел 25.3).
const _: () = assert!(USED <= BUDGET, "буферы драйверов не помещаются в бюджет ОЗУ");

// А число лишних байт покажет ошибка типов: массив должен быть пустым,
// и rustc напишет, какой длины он получился.
const _: [(); 0] = [(); USED.saturating_sub(BUDGET)];
// ANCHOR_END: check
