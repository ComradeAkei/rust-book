//! Задача D: буферы драйверов против бюджета ОЗУ — проверка при сборке.

use core::mem::size_of;

/// Десятичное число из строки окружения сборки.
pub const fn parse_budget(s: &str) -> usize {
    // TODO(задача D): паника «CH25_RAM_BUDGET: не десятичное число» на
    // всём, кроме цифр.
    let _ = s;
    todo!()
}

/// Сколько байт ОЗУ отдано буферам драйверов.
// TODO(задача D): parse_budget(env!("CH25_RAM_BUDGET")), когда build.rs
// начнёт задавать эту переменную.
pub const BUDGET: usize = 0;

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

/// Реестр драйверов станции.
pub const DRIVERS: [Driver; 4] = [
    Driver { name: "uart", ram: size_of::<UartBuffers>() },
    Driver { name: "adc", ram: size_of::<AdcSamples>() },
    Driver { name: "log", ram: size_of::<EventLog>() },
    Driver { name: "radio", ram: size_of::<RadioQueue>() },
];
// ANCHOR_END: drivers

/// Сумма буферов.
pub const fn total(drivers: &[Driver]) -> usize {
    // TODO(задача D)
    let _ = drivers;
    0
}

pub const USED: usize = total(&DRIVERS);

// TODO(задача D): две проверки при компиляции — USED не больше BUDGET с
// текстом «буферы драйверов не помещаются в бюджет ОЗУ» и вторая, которая
// показывает в ошибке число лишних байт.
