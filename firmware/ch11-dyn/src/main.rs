//! Глава 11 на Cortex-M0, M3 и M4F: объекты `dyn Sensor` и их таблицы
//! методов. Таблица датчиков — `static`, и объекты, и таблицы лежат во
//! flash. У объекта на стеке данные в RAM, а таблица всё равно во flash:
//! её строит компилятор, а не программа.
//!
//! ```text
//! cd firmware && cargo run --release -p ch11-dyn
//! ```
#![no_std]
#![no_main]

use ch11::fat::{Counter, Sensor, Thermo};
use ch11::vtable::SENSORS;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

/// RAM у всех трёх машин начинается с 0x2000_0000, flash — с нуля.
fn place(addr: usize) -> &'static str {
    if addr < 0x2000_0000 { "во flash" } else { "в RAM" }
}

/// Адрес данных и адрес таблицы. Что таблица — второе слово, rustc 1.97
/// делает, но не обещает; стабильного способа получить её адрес нет.
fn parts(s: &dyn Sensor) -> (usize, usize) {
    // SAFETY: толстый указатель — два слова, оба читаются как числа.
    unsafe { core::mem::transmute::<&dyn Sensor, (usize, usize)>(s) }
}

// ANCHOR: main
#[entry]
fn main() -> ! {
    hprintln!(
        "&dyn Sensor: {} байт, &Thermo: {} байта",
        size_of::<&dyn Sensor>(),
        size_of::<&Thermo>()
    );
    let local = Counter { pulses: 3 };
    let all: [&dyn Sensor; 3] = [SENSORS[0], SENSORS[1], &local];
    let (mut data_flash, mut tables_flash) = (0, 0);
    for s in all {
        let (data, table) = parts(s);
        hprintln!(
            "{} = {}: {} байта, данные {:#010x} {}, таблица {:#010x} {}",
            s.name(),
            s.read(),
            size_of_val(s),
            data,
            place(data),
            table,
            place(table)
        );
        data_flash += usize::from(data < 0x2000_0000);
        tables_flash += usize::from(table < 0x2000_0000);
    }
    hprintln!("данных во flash: {} из 3, таблиц во flash: {} из 3", data_flash, tables_flash);
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
