// Раздел 26.1: std для thumbv6m-none-eabi нет вовсе. Так выглядит и
// зависимость, которая по умолчанию включает свою возможность std.
#![no_std]

extern crate std;

pub fn nothing() {}
