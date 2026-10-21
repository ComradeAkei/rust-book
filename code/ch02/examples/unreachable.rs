//! Нарушенное обещание `assert_unchecked`: код операции 9 при обещанных
//! `op < 8`. Это неопределённое поведение, и результат зависит от сборки.
//!
//! ```console
//! $ cargo run -p ch02 --example unreachable
//! $ cargo run -p ch02 --release --example unreachable
//! $ cargo +nightly miri run -p ch02 --example unreachable
//! ```

// ANCHOR: unreachable
use ch02::lowering::ch02_exec_assume;
use std::hint::black_box;

fn main() {
    let op = black_box(9);
    // SAFETY: нарушено намеренно: функция требует op < 8.
    let r = unsafe { ch02_exec_assume(op, 7, 5) };
    println!("ch02_exec_assume(9, 7, 5) = {r}");
}
// ANCHOR_END: unreachable
