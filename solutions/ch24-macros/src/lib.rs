//! Глава 24: эталонные процедурные макросы задач B–D. Пользоваться ими
//! нужно через крейт `ch24-solutions`, который их реэкспортирует.
//!
//! - [`Bitfield`] — B: упаковка полей с `#[bits(N)]` в слово `u32`.
//! - [`timed`] — C: замер времени функции охранником с `Drop`.
//! - [`include_svd!`] — D: API регистров периферии из файла SVD.

mod bitfield;
mod svd;
mod timed;

use proc_macro::TokenStream;

/// `#[derive(Bitfield)]`: рядом со структурой — `{Имя}Bits(pub u32)` с
/// геттерами, `set_*`, `with_*`, сдвигами и масками полей.
#[proc_macro_derive(Bitfield, attributes(bits))]
pub fn derive_bitfield(input: TokenStream) -> TokenStream {
    bitfield::derive(input)
}

/// `#[timed(STATS)]`: время каждого вызова функции копится в `STATS`.
#[proc_macro_attribute]
pub fn timed(attr: TokenStream, item: TokenStream) -> TokenStream {
    match timed::expand(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

/// `include_svd!("файл.svd", "UART0")`: модуль `uart0` с блоком регистров.
/// Путь — от каталога крейта, который вызывает макрос.
#[proc_macro]
pub fn include_svd(input: TokenStream) -> TokenStream {
    match svd::expand(input.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
