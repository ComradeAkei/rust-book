//! 24.4: `#[interrupt]` — обработчик прерывания в духе cortex-m-rt.
//! Макрос проверяет сигнатуру за человека, сверяет имя со списком
//! прерываний устройства, превращает `static mut` в начале тела в `&mut` и
//! экспортирует переходник под именем прерывания.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Error, FnArg, Item, ItemFn, ItemStatic, ReturnType, Signature, StaticMutability, Stmt, Type,
    Visibility,
};

fn push(errors: &mut Option<Error>, e: Error) {
    match errors {
        Some(all) => all.combine(e),
        None => *errors = Some(e),
    }
}

// ANCHOR: check
/// Все нарушения сразу, у каждого — спан той части сигнатуры, которую надо
/// исправить.
fn check(vis: &Visibility, sig: &Signature) -> syn::Result<()> {
    let mut errors = None;
    if !matches!(vis, Visibility::Inherited) {
        push(
            &mut errors,
            Error::new_spanned(vis, "обработчик вызывает только процессор: уберите видимость"),
        );
    }
    if let Some(c) = &sig.constness {
        push(&mut errors, Error::new_spanned(c, "обработчик прерывания не может быть `const fn`"));
    }
    if let Some(a) = &sig.asyncness {
        push(
            &mut errors,
            Error::new_spanned(
                a,
                "обработчик прерывания не может быть `async fn`: ждать в нём некому",
            ),
        );
    }
    if let Some(abi) = &sig.abi {
        push(
            &mut errors,
            Error::new_spanned(abi, "ABI переходника задаёт макрос: напишите просто `fn`"),
        );
    }
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        push(
            &mut errors,
            Error::new_spanned(&sig.generics, "у обработчика прерывания нет параметров типа"),
        );
    }
    if !sig.inputs.is_empty() {
        push(
            &mut errors,
            Error::new_spanned(
                &sig.inputs,
                "обработчик прерывания не принимает аргументов: состояние — `static mut` в начале тела",
            ),
        );
    }
    if let ReturnType::Type(_, ty) = &sig.output {
        let unit = matches!(&**ty, Type::Tuple(t) if t.elems.is_empty());
        if !unit && !matches!(&**ty, Type::Never(_)) {
            push(
                &mut errors,
                Error::new_spanned(ty, "обработчик прерывания возвращает `()` или `!`"),
            );
        }
    }
    errors.map_or(Ok(()), Err)
}
// ANCHOR_END: check

/// `static mut` из начала тела — состояние обработчика.
fn split_statics(stmts: Vec<Stmt>) -> (Vec<ItemStatic>, Vec<Stmt>) {
    let mut statics = Vec::new();
    let mut rest = stmts.into_iter().peekable();
    while let Some(Stmt::Item(Item::Static(s))) = rest.peek() {
        if !matches!(s.mutability, StaticMutability::Mut(_)) {
            break;
        }
        let Some(Stmt::Item(Item::Static(s))) = rest.next() else { unreachable!() };
        statics.push(s);
    }
    (statics, rest.collect())
}

// ANCHOR: expand
pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(Error::new_spanned(attr, "#[interrupt] не принимает аргументов"));
    }
    let mut f: ItemFn = syn::parse2(item)?;
    check(&f.vis, &f.sig)?;

    let ident = f.sig.ident.clone();
    let export = ident.to_string();
    let inner = format_ident!("__ch24_{}", ident);
    let trampoline = format_ident!("__ch24_{}_trampoline", ident);

    let (statics, stmts) = split_statics(std::mem::take(&mut f.block.stmts));
    let params = statics.iter().map(|s| {
        let (attrs, name, ty) = (&s.attrs, &s.ident, &s.ty);
        let arg: FnArg = syn::parse_quote!(#[allow(non_snake_case)] #(#attrs)* #name: &mut #ty);
        arg
    });
    f.sig.inputs = params.collect();
    f.sig.ident = inner.clone();
    // Имя сверяется со списком прерываний устройства: путь `interrupt::`
    // разрешается там, где стоит атрибут, — как у cortex-m-rt.
    f.block.stmts =
        std::iter::once(syn::parse_quote! { let _ = interrupt::#ident; }).chain(stmts).collect();

    let resources = statics.iter().map(|s| {
        let (attrs, name, ty, expr) = (&s.attrs, &s.ident, &s.ty, &s.expr);
        quote! {{
            #(#attrs)*
            static mut #name: #ty = #expr;
            // SAFETY: static объявлен в этом блоке, и другого пути к нему
            // нет; процессор не вызывает обработчик повторно, пока тот не
            // вернулся, поэтому ссылка единственная.
            unsafe { &mut *&raw mut #name }
        }}
    });

    // Имена прерываний — заглавные, как в техническом описании; новым
    // именам функций достаётся их спан, и с ним — линт non_snake_case.
    Ok(quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        #[unsafe(export_name = #export)]
        pub unsafe extern "C" fn #trampoline() {
            #inner(#(#resources),*)
        }

        #[allow(non_snake_case)]
        #f
    })
}
// ANCHOR_END: expand
