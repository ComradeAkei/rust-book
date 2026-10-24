// Задача 25.A: шесть объявлений. В какую секцию объектного файла попадёт
// каждое под thumbv7m-none-eabi — .rodata, .data или .bss? Тест собирает
// этот файл закреплённым rustc (`--emit obj -C opt-level=s`) и сверяет
// ответы из `a_sections::ANSWERS`. `#[unsafe(no_mangle)]` сохраняет имена
// и не даёт выбросить неиспользуемые объекты.
#![no_std]

use core::sync::atomic::AtomicU32;

// ANCHOR: decls
#[unsafe(no_mangle)]
pub static GAINS: [u16; 4] = [100, 200, 400, 800];

#[unsafe(no_mangle)]
pub static ZEROS: [u32; 16] = [0; 16];

#[unsafe(no_mangle)]
pub static BOOTS: AtomicU32 = AtomicU32::new(1);

#[unsafe(no_mangle)]
pub static EVENTS: AtomicU32 = AtomicU32::new(0);

#[unsafe(no_mangle)]
pub static mut SCRATCH: [u8; 64] = [0; 64];

#[unsafe(no_mangle)]
pub static mut CALIB: [i16; 3] = [12, -7, 3];
// ANCHOR_END: decls
