//! Задача C: `#[timed(STATS)]` — замер времени функции. В начало тела
//! вставляется охранник `STATS.start()`; его `Drop` записывает время в
//! `STATS`, как бы функция ни завершилась: обычным возвратом, `return`,
//! `?` или паникой с раскруткой стека.

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Error, ItemFn, Path};

// ANCHOR: expand
pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if attr.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "#[timed] ждёт путь к static Timing: #[timed(STATS)]",
        ));
    }
    let stats: Path = syn::parse2(attr)?;
    let mut f: ItemFn = syn::parse2(item)?;

    let mut errors: Option<Error> = None;
    let mut push = |e: Error| match &mut errors {
        Some(all) => all.combine(e),
        None => errors = Some(e),
    };
    if let Some(c) = &f.sig.constness {
        push(Error::new_spanned(c, "#[timed] на const fn: при вычислении в const часов нет"));
    }
    if let Some(a) = &f.sig.asyncness {
        push(Error::new_spanned(
            a,
            "#[timed] на async fn замерил бы время от первого poll до готовности вместе с ожиданием в .await, а не работу функции",
        ));
    }
    if let Some(e) = errors {
        return Err(e);
    }

    // Имя охранника с гигиеной mixed_site: аргумент функции с тем же
    // именем остаётся виден её телу. Подчёркивание глушит линт
    // unused_variables, но, в отличие от `let _ =`, охранник живёт до конца
    // тела.
    let guard = Ident::new("_guard", Span::mixed_site());
    let body = &f.block;
    f.block = syn::parse_quote!({
        let #guard = #stats.start();
        #body
    });
    Ok(f.into_token_stream())
}
// ANCHOR_END: expand
