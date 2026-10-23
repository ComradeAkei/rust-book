//! Задача C: `#[timed(STATS)]` — замер времени функции.
//!
//! Заготовка разбирает аргумент и функцию и возвращает функцию как есть.
//! Допишите: охранник `STATS.start()` в начале тела (он должен дожить до
//! конца тела и не мешать аргументам функции с любыми именами) и ошибки со
//! спанами для `const fn`, `async fn` и атрибута без аргумента.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{ItemFn, Path};

// ANCHOR: skeleton
pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let _stats: Path = syn::parse2(attr)?;
    let f: ItemFn = syn::parse2(item)?;
    // не решено: раздел 24.7, задача C
    Ok(f.into_token_stream())
}
// ANCHOR_END: skeleton
