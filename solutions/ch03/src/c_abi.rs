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
