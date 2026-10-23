//! Одна и та же функция из ch08 на Cortex-M0 и Cortex-M3 и данные,
//! сгенерированные build.rs.
#![no_std]
#![no_main]

use ch08::atomic::ch08_bump;
use ch08::generated::{SENSORS, build, sensor};
use ch08_sections as _;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

static EVENTS: AtomicU32 = AtomicU32::new(41);

// ANCHOR: main
#[entry]
fn main() -> ! {
    let how = if cfg!(target_has_atomic = "32") {
        "LDREX/STREX"
    } else {
        "запрет прерываний"
    };
    let was = ch08_bump(&EVENTS);
    hprintln!("ch08_bump ({}): было {}, стало {}", how, was, EVENTS.load(Relaxed));
    hprintln!(
        "build.rs работал на {}, cfg!(target_arch = \"arm\") = {}",
        build::BUILD_RS_ARCH,
        build::BUILD_RS_CFG_ARM
    );
    hprintln!("крейт собран для {}", build::TARGET_ARCH);
    for s in &SENSORS {
        hprintln!("датчик {:2} {:10} ×{} {}", s.id, s.name, s.scale, s.unit);
    }
    hprintln!("sensor(7) по адресу {:p}", sensor(7).unwrap());
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main
