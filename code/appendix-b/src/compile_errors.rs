//! Код, который компилировался в edition 2021 и не компилируется в 2024.
//!
//! Каждый фрагмент проверен doc-тестом `compile_fail`. Коды ошибок rustdoc
//! сверяет только на nightly, поэтому CI запускает doc-тесты ещё и через
//! `cargo +nightly test --doc`. Для книги те же фрагменты продублированы в
//! комментариях с якорями вместе с первой строкой сообщения rustc 1.97.0.
//!
//! Блок `extern` без `unsafe`:
//!
//! ```compile_fail
//! extern "C" {
//!     fn getpid() -> i32;
//! }
//! ```
//!
//! Небезопасный атрибут без `unsafe(...)`:
//!
//! ```compile_fail
//! #[no_mangle]
//! pub extern "C" fn tick() {}
//! ```
//!
//! Небезопасная операция в теле `unsafe fn` без своего блока (в 2024 это
//! предупреждение, с `deny` — ошибка):
//!
//! ```compile_fail,E0133
//! #![deny(unsafe_op_in_unsafe_fn)]
//! unsafe fn read(p: *const u32) -> u32 {
//!     *p
//! }
//! ```
//!
//! Ссылка на `static mut`:
//!
//! ```compile_fail
//! static mut COUNTER: u32 = 0;
//! fn get() -> &'static u32 {
//!     unsafe { &COUNTER }
//! }
//! ```
//!
//! `set_var` стала `unsafe fn`:
//!
//! ```compile_fail,E0133
//! std::env::set_var("TELEMETRY_PORT", "4242");
//! ```
//!
//! Тип `!` больше не превращается в `()`:
//!
//! ```compile_fail,E0277
//! fn f<T: Default>() -> Result<T, ()> {
//!     Ok(T::default())
//! }
//! fn g() -> Result<(), ()> {
//!     f()?;
//!     Ok(())
//! }
//! ```
//!
//! `gen` стало зарезервированным словом:
//!
//! ```compile_fail
//! let gen = 1;
//! ```
//!
//! `mut` в образце, который неявно берёт ссылку:
//!
//! ```compile_fail
//! fn bumped(opt: &Option<u32>) -> Option<u32> {
//!     if let Some(mut x) = opt {
//!         x += 1;
//!         Some(x)
//!     } else {
//!         None
//!     }
//! }
//! ```
//!
//! `impl Trait` захватывает время жизни, и заимствование тянется дальше:
//!
//! ```compile_fail,E0502
//! fn indices(v: &[u8]) -> impl Iterator<Item = usize> {
//!     0..v.len()
//! }
//! let mut buf = vec![1u8, 2, 3];
//! let it = indices(&buf);
//! buf.push(4);
//! assert_eq!(it.count(), 3);
//! ```

/* ANCHOR: extern_block
extern "C" {
    fn getpid() -> i32;
}
// error: extern blocks must be unsafe
ANCHOR_END: extern_block */

/* ANCHOR: no_mangle
#[no_mangle]
pub extern "C" fn tick() {}
// error: unsafe attribute used without unsafe
ANCHOR_END: no_mangle */

/* ANCHOR: unsafe_op
unsafe fn read(p: *const u32) -> u32 {
    *p
}
// warning[E0133]: dereference of raw pointer is unsafe and requires unsafe block
// note: an unsafe function restricts its caller, but its body is safe by default
ANCHOR_END: unsafe_op */

/* ANCHOR: static_mut_ref
static mut COUNTER: u32 = 0;
fn get() -> &'static u32 {
    unsafe { &COUNTER }
}
// error: creating a shared reference to mutable static
// help: use `&raw const` instead to create a raw pointer
ANCHOR_END: static_mut_ref */

/* ANCHOR: set_var
std::env::set_var("TELEMETRY_PORT", "4242");
// error[E0133]: call to unsafe function `set_var` is unsafe and requires unsafe block
ANCHOR_END: set_var */

/* ANCHOR: never
fn f<T: Default>() -> Result<T, ()> {
    Ok(T::default())
}
fn g() -> Result<(), ()> {
    f()?;   // какой T? Раньше (), теперь !
    Ok(())
}
// error[E0277]: the trait bound `!: Default` is not satisfied
ANCHOR_END: never */

/* ANCHOR: binding
if let Some(mut x) = opt {      // opt: &Option<u32>
// error: cannot mutably bind by value within an implicitly-borrowing pattern
ANCHOR_END: binding */

/* ANCHOR: overcapture
fn indices(v: &[u8]) -> impl Iterator<Item = usize> {
    0..v.len()
}
let mut buf = vec![1u8, 2, 3];
let it = indices(&buf);
buf.push(4);
// error[E0502]: cannot borrow `buf` as mutable because it is also borrowed as immutable
// note: this call may capture more lifetimes than intended, because Rust 2024
//       has adjusted the `impl Trait` lifetime capture rules
assert_eq!(it.count(), 3);
ANCHOR_END: overcapture */
