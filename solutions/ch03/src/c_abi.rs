//! Задача 3.C. Когда структура перестаёт помещаться в регистры.
//!
//! Все размеры — в байтах, для структур `#[repr(C)] struct W(pub [u32; N])`
//! с N от 1 до 48, то есть от 4 до 192 байт.

pub use ch03::task_c::Thresholds;

// ANCHOR: answers
/// Rust ABI на x86_64: агрегат размером до машинного слова идёт целым
/// числом в регистре, всё большее — указателем на копию. И для аргумента,
/// и для результата (через скрытый параметр sret).
pub const X86_64_RUST: Thresholds = Thresholds { arg_in_regs: 8, ret_in_regs: 8 };

/// System V AMD64: до 16 байт — классы INTEGER в двух регистрах (rdi:rsi
/// для аргумента, rax:rdx для результата), больше — класс MEMORY: копия в
/// области аргументов на стеке и sret для результата.
pub const X86_64_C: Thresholds = Thresholds { arg_in_regs: 16, ret_in_regs: 16 };

/// Rust ABI на Cortex-M: то же правило «не больше машинного слова», но
/// слово здесь 4 байта.
pub const THUMB_RUST: Thresholds = Thresholds { arg_in_regs: 4, ret_in_regs: 4 };

/// AAPCS: аргумент раскладывается по словам в r0–r3, остаток — на стек,
/// так что целиком в регистрах помещаются 16 байт. Составной результат
/// больше 4 байт возвращается через память, даже 8-байтовый.
pub const THUMB_C: Thresholds = Thresholds { arg_in_regs: 16, ret_in_regs: 4 };

/// С какого размера копия для передачи `W` по значению из `&W`
/// становится вызовом `memcpy`: до 128 байт LLVM копирует инлайн
/// 16-байтовыми movups, дальше зовёт memcpy.
pub const X86_64_MEMCPY_FROM: usize = 132;

/// На Cortex-M4 инлайн-копия идёт парами LDRD/STRD или LDM/STM до 64
/// байт, дальше — вызов `__aeabi_memcpy4`.
pub const THUMB_MEMCPY_FROM: usize = 68;
// ANCHOR_END: answers

// ANCHOR: probes
/// Пробные функции для опытов, по образцу пробного крейта из теста:
/// `cargo asm -p ch03-solutions --lib --llvm ch03c_arg_rust_3`. Модуль
/// `wN` содержит структуру из N слов и функции с ней в аргументе и в
/// результате для обоих ABI, а также вызывающего, которому приходится
/// делать копию: вызываемая функция меняет свой аргумент.
macro_rules! probe {
    ($m:ident, $n:literal) => {
        pub mod $m {
            use core::hint::black_box;

            #[repr(C)]
            pub struct W(pub [u32; $n]);

            #[inline(never)]
            #[unsafe(export_name = concat!("ch03c_arg_rust_", $n))]
            pub fn arg_rust(mut s: W) -> u32 {
                s.0.reverse();
                black_box(&mut s);
                s.0[0]
            }

            #[inline(never)]
            #[unsafe(export_name = concat!("ch03c_arg_c_", $n))]
            pub extern "C" fn arg_c(mut s: W) -> u32 {
                s.0.reverse();
                black_box(&mut s);
                s.0[0]
            }

            #[inline(never)]
            #[unsafe(export_name = concat!("ch03c_ret_rust_", $n))]
            pub fn ret_rust(x: u32) -> W {
                W([x; $n])
            }

            #[inline(never)]
            #[unsafe(export_name = concat!("ch03c_ret_c_", $n))]
            pub extern "C" fn ret_c(x: u32) -> W {
                W([x; $n])
            }

            #[inline(never)]
            #[unsafe(export_name = concat!("ch03c_call_rust_", $n))]
            pub fn call_rust(s: &W) -> u32 {
                arg_rust(W(s.0))
            }

            #[inline(never)]
            #[unsafe(export_name = concat!("ch03c_call_c_", $n))]
            pub fn call_c(s: &W) -> u32 {
                arg_c(W(s.0))
            }
        }
    };
}

probe!(w1, 1);
probe!(w2, 2);
probe!(w3, 3);
probe!(w4, 4);
probe!(w5, 5);
probe!(w16, 16);
probe!(w17, 17);
probe!(w32, 32);
probe!(w33, 33);
probe!(w1024, 1024);
// ANCHOR_END: probes
