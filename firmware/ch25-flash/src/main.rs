//! Глава 25, раздел 25.6: что вычислено при компиляции и чем за это
//! платит ОЗУ.
//!
//! Прошивка печатает границы `.data` и `.bss` из символов скрипта
//! компоновки cortex-m-rt — сколько байт код запуска скопировал из flash
//! и сколько обнулил до `main`, — адреса таблиц, посчитанных `const fn`, и
//! пользуется ими: делитель UART0 из константы записан в регистры
//! lm3s6965 и прочитан обратно, отсчёты АЦП переведены в единицы датчика
//! таблицей обратных вместо деления.
//!
//! ```text
//! cd firmware
//! cargo run --release -p ch25-flash
//! cargo run --release -p ch25-flash --features ram-table
//! cargo run --release -p ch25-flash --target thumbv6m-none-eabi
//! ```
#![no_std]
#![no_main]

use core::num::NonZeroU8;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

use ch25::const_fn::CLASSES;
use ch25::tables::{RECIP, ch25_scale_div, ch25_scale_recip};

// ANCHOR: linker
// Границы секций из link.x cortex-m-rt: адреса, а не переменные.
unsafe extern "C" {
    static __sdata: u8;
    static __edata: u8;
    static __sidata: u8;
    static __sbss: u8;
    static __ebss: u8;
}

fn span(start: *const u8, end: *const u8) -> (usize, usize) {
    (start as usize, end as usize - start as usize)
}
// ANCHOR_END: linker

// ANCHOR: ram
/// Счётчик загрузок с ненулевым начальным значением: в `.data`.
static BOOTS: AtomicU32 = AtomicU32::new(1);

/// Буфер журнала, изначально нулевой: в `.bss`.
static mut LOG: [u8; 64] = [0; 64];

/// Та же таблица обратных в ОЗУ: только с возможностью `ram-table`.
#[cfg(feature = "ram-table")]
static mut RECIP_RAM: [u32; 256] = ch25::tables::reciprocals();
// ANCHOR_END: ram

/// Регион по карте памяти из `memory.x`.
fn region(addr: usize) -> &'static str {
    match addr {
        0x0000_0000..0x0004_0000 => "flash",
        0x2000_0000..0x2000_4000 => "ОЗУ",
        _ => "?",
    }
}

fn show<T: ?Sized>(what: &str, p: *const T, size: usize) {
    let addr = p.cast::<u8>() as usize;
    hprintln!("{:<22} {:#010x}  {:<5} {:>5} байт", what, addr, region(addr), size);
}

// ANCHOR: uart
/// Cortex-M3 (номер ядра 0xC23 в CPUID) у нас только на lm3s6965evb. У
/// micro:bit и MPS2 AN386 по адресу UART0 lm3s6965 другое или ничего.
fn is_lm3s6965() -> bool {
    // SAFETY: CPUID — регистр SCB, он есть у любого Cortex-M.
    let cpuid = unsafe { (0xE000_ED00 as *const u32).read_volatile() };
    (cpuid >> 4) & 0xFFF == 0xC23
}

/// Записать делитель, посчитанный при компиляции, и прочитать обратно.
fn program_uart0() -> (u32, u32) {
    use ch25::const_fn::UART0;
    // UART0 lm3s6965 (PL011): делители скорости.
    const UART0_IBRD: *mut u32 = 0x4000_C024 as *mut u32;
    const UART0_FBRD: *mut u32 = 0x4000_C028 as *mut u32;
    // SAFETY: адреса регистров PL011 из карты памяти LM3S6965; UART0 в
    // этот момент никто не использует.
    unsafe {
        UART0_IBRD.write_volatile(UART0.ibrd as u32);
        UART0_FBRD.write_volatile(UART0.fbrd as u32);
        (UART0_IBRD.read_volatile(), UART0_FBRD.read_volatile())
    }
}
// ANCHOR_END: uart

#[entry]
fn main() -> ! {
    // ANCHOR: startup
    let (data, data_len) = span(&raw const __sdata, &raw const __edata);
    let (bss, bss_len) = span(&raw const __sbss, &raw const __ebss);
    let image = &raw const __sidata as usize;
    hprintln!(".data: {} байт в ОЗУ с {:#010x}, образ во flash с {:#010x}", data_len, data, image);
    hprintln!(".bss:  {} байт в ОЗУ с {:#010x}, обнулены до main", bss_len, bss);
    // ANCHOR_END: startup

    show("static CLASSES", &CLASSES, size_of_val(&CLASSES));
    show("static RECIP", &RECIP, size_of_val(&RECIP));
    show("static BOOTS (.data)", &BOOTS, size_of::<AtomicU32>());
    show("static mut LOG (.bss)", &raw const LOG, 64);
    #[cfg(feature = "ram-table")]
    show("static mut RECIP_RAM", &raw const RECIP_RAM, 1024);
    BOOTS.store(BOOTS.load(Relaxed) + 1, Relaxed);

    if is_lm3s6965() {
        let (ibrd, fbrd) = program_uart0();
        let ppm = ch25::const_fn::UART0.error_ppm;
        hprintln!(
            "UART0 при 16 МГц, 115200 бод: IBRD = {}, FBRD = {}, ошибка {} ppm",
            ibrd,
            fbrd,
            ppm
        );
    }

    // ANCHOR: scale
    // Отсчёты АЦП и калибровочные делители датчиков.
    let samples = [(3000u32, 7u8), (4095, 255), (1234, 13), (2048, 3)];
    let mut same = 0;
    for (code, k) in samples {
        let k = NonZeroU8::new(k).unwrap();
        if ch25_scale_recip(code, k) == ch25_scale_div(code, k) {
            same += 1;
        }
    }
    let (code, k) = (3000, NonZeroU8::new(7).unwrap());
    hprintln!(
        "отсчёт {} / {}: таблица {}, деление {}",
        code,
        k,
        ch25_scale_recip(code, k),
        ch25_scale_div(code, k)
    );
    hprintln!("таблица и деление совпали: {} из {}", same, samples.len());
    // ANCHOR_END: scale
    hprintln!("BOOTS = {}", BOOTS.load(Relaxed));

    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
