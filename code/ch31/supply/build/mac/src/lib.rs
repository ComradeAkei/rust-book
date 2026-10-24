// ANCHOR: mac
//! Процедурный макрос — тоже программа, и выполняется она внутри
//! компилятора: при `cargo check` и при каждом анализе в rust-analyzer.
//! Этот макрос ничего не порождает, только оставляет след рядом с крейтом.
use proc_macro::TokenStream;

#[proc_macro]
pub fn stamp(_: TokenStream) -> TokenStream {
    let mark = concat!(env!("CARGO_MANIFEST_DIR"), "/../proc-macro-ran");
    std::fs::write(mark, "процедурный макрос").unwrap();
    TokenStream::new()
}
// ANCHOR_END: mac
