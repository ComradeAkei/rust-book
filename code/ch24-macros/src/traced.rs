//! 24.3: `#[traced]` — атрибут, который вставляет в начало тела функции
//! свою локальную переменную. С каким спаном создано её имя, решает,
//! увидит ли её код пользователя.

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Error, ItemFn, LitStr};

// ANCHOR: traced
pub fn expand(attr: TokenStream, item: TokenStream, hygiene: Span) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(Error::new_spanned(attr, "#[traced] не принимает аргументов"));
    }
    let mut f: ItemFn = syn::parse2(item)?;
    let fn_name = LitStr::new(&f.sig.ident.to_string(), f.sig.ident.span());
    // Имя переменной макроса. Span::call_site() — как если бы его написал
    // пользователь; Span::mixed_site() — гигиена macro_rules! для локальных
    // переменных: имя видно только токенам самого макроса.
    let name = Ident::new("name", hygiene);
    let body = &f.block;
    f.block = syn::parse_quote!({
        let #name: &'static str = #fn_name;
        ::ch24::trace::enter(#name);
        #body
    });
    Ok(f.into_token_stream())
}
// ANCHOR_END: traced
