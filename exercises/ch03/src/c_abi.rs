//! Задача 3.C. Когда структура перестаёт помещаться в регистры.
//!
//! Все размеры — в байтах, для структур `#[repr(C)] struct W(pub [u32; N])`
//! с N от 1 до 48. Пороги найдите опытами: напишите функции, которые
//! принимают и возвращают такие структуры, и посмотрите их LLVM IR и
//! ассемблер (раздел 3.5 и приложение A):
//!
//! ```text
//! cargo asm -p ch03-exercises --lib --llvm имя_функции
//! cargo asm -p ch03-exercises --lib --target thumbv7em-none-eabihf имя_функции
//! ```
//!
//! Тест `cargo test -p ch03-exercises task_c` сам соберёт структуры всех
//! 48 размеров для обеих целей и покажет первое расхождение с вашими
//! ответами.

pub use ch03::task_c::Thresholds;

/// Rust ABI (обычная `fn`) на x86_64.
pub const X86_64_RUST: Thresholds = Thresholds { arg_in_regs: 0, ret_in_regs: 0 };

/// `extern "C"` на x86_64 (System V AMD64).
pub const X86_64_C: Thresholds = Thresholds { arg_in_regs: 0, ret_in_regs: 0 };

/// Rust ABI на thumbv7em-none-eabihf (Cortex-M4F).
pub const THUMB_RUST: Thresholds = Thresholds { arg_in_regs: 0, ret_in_regs: 0 };

/// `extern "C"` на thumbv7em-none-eabihf (AAPCS).
pub const THUMB_C: Thresholds = Thresholds { arg_in_regs: 0, ret_in_regs: 0 };

/// Наименьший размер, при котором копия для вызова `fn f(w: W)` из
/// `&W` (Rust ABI, `f` меняет свой аргумент) становится вызовом `memcpy`.
pub const X86_64_MEMCPY_FROM: usize = 0;

/// То же на thumbv7em-none-eabihf.
pub const THUMB_MEMCPY_FROM: usize = 0;
