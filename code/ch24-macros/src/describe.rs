//! 24.2–24.4: `#[derive(Describe)]` — описание раскладки структуры
//! (имена, типы, смещения и размеры полей) и дамп значений, с
//! атрибутами-помощниками `#[describe(...)]`.
//!
//! Логика написана на proc_macro2 и syn и не трогает `proc_macro`: её можно
//! вызывать из обычных модульных тестов (раздел 24.5).

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Error, Index, LitInt, LitStr, Member, Path, Type};

/// Откуда брать спаны сгенерированного кода: от полей пользователя или от
/// места вызова. Второй вариант — `#[derive(DescribeCallSite)]`, образец
/// плохих сообщений об ошибках из раздела 24.3.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Spans {
    User,
    CallSite,
}

// ANCHOR: opts
/// Атрибуты-помощники структуры: `#[describe(size = 8, crate = путь)]`.
#[derive(Default)]
struct StructOpts {
    size: Option<LitInt>,
    krate: Option<Path>,
}

fn struct_opts(attrs: &[Attribute]) -> syn::Result<StructOpts> {
    let mut opts = StructOpts::default();
    let mut errors = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("describe")) {
        let parsed = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("size") {
                opts.size = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("crate") {
                opts.krate = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("skip") {
                return Err(meta.error("`skip` ставится на поле, а не на структуру"));
            } else {
                return Err(
                    meta.error("неизвестный ключ: у структуры есть `size = N` и `crate = путь`")
                );
            }
            Ok(())
        });
        // Ошибка в одном атрибуте не мешает разобрать следующие.
        if let Err(e) = parsed {
            push(&mut errors, e);
        }
    }
    errors.map_or(Ok(opts), Err)
}

/// Атрибут-помощник поля: `#[describe(skip)]`.
fn field_skipped(attrs: &[Attribute]) -> syn::Result<bool> {
    let mut skip = false;
    for attr in attrs.iter().filter(|a| a.path().is_ident("describe")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
                Ok(())
            } else {
                Err(meta.error("неизвестный ключ: у поля есть только `skip`"))
            }
        })?;
    }
    Ok(skip)
}
// ANCHOR_END: opts

/// Ошибки копятся: пользователь видит все сразу, а не по одной за сборку.
fn push(errors: &mut Option<Error>, e: Error) {
    match errors {
        Some(all) => all.combine(e),
        None => *errors = Some(e),
    }
}

struct DescribedField<'a> {
    member: Member,
    label: LitStr,
    ty: &'a Type,
}

// ANCHOR: expand
pub fn expand(input: DeriveInput, spans: Spans) -> syn::Result<TokenStream> {
    let data = match &input.data {
        Data::Struct(s) => s,
        Data::Enum(e) => {
            return Err(Error::new_spanned(
                e.enum_token,
                "Describe: только для структур, не для перечислений",
            ));
        }
        Data::Union(u) => {
            return Err(Error::new_spanned(
                u.union_token,
                "Describe: только для структур, не для объединений",
            ));
        }
    };
    let mut errors = None;
    let opts = struct_opts(&input.attrs).unwrap_or_else(|e| {
        push(&mut errors, e);
        StructOpts::default()
    });

    let mut fields = Vec::new();
    for (i, f) in data.fields.iter().enumerate() {
        match field_skipped(&f.attrs) {
            Ok(true) => continue,
            Ok(false) => {}
            Err(e) => push(&mut errors, e),
        }
        let member = match &f.ident {
            Some(ident) => Member::Named(ident.clone()),
            None => Member::Unnamed(Index::from(i)),
        };
        let label = match &f.ident {
            Some(ident) => LitStr::new(&ident.to_string(), ident.span()),
            None => LitStr::new(&i.to_string(), f.ty.span()),
        };
        fields.push(DescribedField { member, label, ty: &f.ty });
    }
    if let Some(e) = errors {
        return Err(e);
    }

    // У процедурного макроса нет `$crate`: путь к крейту с трейтом
    // приходится писать от корня или брать у пользователя.
    let krate = opts.krate.unwrap_or_else(|| syn::parse_quote!(::ch24));
    let name = &input.ident;
    let name_str = LitStr::new(&name.to_string(), name.span());
    // Как derive(Debug) из std: каждому параметру типа — ограничение
    // `Debug`, иначе `dump` не напечатает поле типа `T`.
    let mut generics = input.generics.clone();
    for param in input.generics.type_params() {
        let ident = &param.ident;
        generics.make_where_clause().predicates.push(syn::parse_quote!(#ident: ::core::fmt::Debug));
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let entries = fields.iter().map(|f| {
        let (member, label, ty) = (&f.member, &f.label, f.ty);
        quote! {
            #krate::Field {
                name: #label,
                ty: ::core::stringify!(#ty),
                offset: ::core::mem::offset_of!(Self, #member),
                size: ::core::mem::size_of::<#ty>(),
            }
        }
    });
    // ANCHOR: dump_spans
    let dumps = fields.iter().enumerate().map(|(i, f)| {
        let member = &f.member;
        let span = match spans {
            Spans::User => f.ty.span(),
            Spans::CallSite => Span::call_site(),
        };
        quote_spanned! {span=>
            #krate::dump_field(out, &<Self as #krate::Describe>::FIELDS[#i], &self.#member)?;
        }
    });
    // ANCHOR_END: dump_spans
    let size_check = opts.size.map(|size| size_check(name, &input.generics, &size));

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #krate::Describe for #name #ty_generics #where_clause {
            const NAME: &'static ::core::primitive::str = #name_str;
            const SIZE: ::core::primitive::usize = ::core::mem::size_of::<Self>();
            const ALIGN: ::core::primitive::usize = ::core::mem::align_of::<Self>();
            const FIELDS: &'static [#krate::Field] = &[#(#entries),*];

            fn dump(&self, out: &mut dyn ::core::fmt::Write) -> ::core::fmt::Result {
                #krate::dump_header::<Self>(out)?;
                #(#dumps)*
                ::core::result::Result::Ok(())
            }
        }
        #size_check
    })
}
// ANCHOR_END: expand

// ANCHOR: size_check
/// `#[describe(size = N)]`: проверка размера при компиляции. Спан — от
/// литерала `N`, поэтому ошибка E0080 указывает в атрибут пользователя.
fn size_check(name: &syn::Ident, generics: &syn::Generics, size: &LitInt) -> TokenStream {
    if !generics.params.is_empty() {
        return Error::new_spanned(size, "size = N: только для структур без параметров")
            .into_compile_error();
    }
    let msg = format!("Describe: размер {name} не равен {}", size.base10_digits());
    quote_spanned! {size.span()=>
        const _: () = ::core::assert!(::core::mem::size_of::<#name>() == #size, #msg);
    }
}
// ANCHOR_END: size_check

/// Входная точка для `#[proc_macro_derive]`: ошибка превращается в
/// `compile_error!` с её спаном.
pub fn derive(input: proc_macro::TokenStream, spans: Spans) -> proc_macro::TokenStream {
    match syn::parse::<DeriveInput>(input) {
        Ok(input) => expand(input, spans).unwrap_or_else(Error::into_compile_error).into(),
        Err(e) => e.into_compile_error().into(),
    }
}

/// Для модульных тестов: строка сгенерированного кода.
#[cfg(test)]
pub fn expand_str(input: TokenStream) -> String {
    let input: DeriveInput = syn::parse2(input).unwrap();
    match expand(input, Spans::User) {
        Ok(ts) => ts.to_string(),
        Err(e) => e.to_compile_error().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::expand_str;
    use quote::quote;

    // ANCHOR: unit
    /// Логика derive на proc_macro2 проверяется обычным тестом: вход —
    /// `quote!`, выход сравнивается с ожидаемым `quote!` как строка токенов.
    #[test]
    fn expands_to_impl() {
        let got = expand_str(quote! {
            struct P { x: u8 }
        });
        let want = quote! {
            #[automatically_derived]
            impl ::ch24::Describe for P {
                const NAME: &'static ::core::primitive::str = "P";
                const SIZE: ::core::primitive::usize = ::core::mem::size_of::<Self>();
                const ALIGN: ::core::primitive::usize = ::core::mem::align_of::<Self>();
                const FIELDS: &'static [::ch24::Field] = &[::ch24::Field {
                    name: "x",
                    ty: ::core::stringify!(u8),
                    offset: ::core::mem::offset_of!(Self, x),
                    size: ::core::mem::size_of::<u8>(),
                }];

                fn dump(&self, out: &mut dyn ::core::fmt::Write) -> ::core::fmt::Result {
                    ::ch24::dump_header::<Self>(out)?;
                    ::ch24::dump_field(out, &<Self as ::ch24::Describe>::FIELDS[0usize], &self.x)?;
                    ::core::result::Result::Ok(())
                }
            }
        };
        assert_eq!(got, want.to_string());
    }

    #[test]
    fn enum_becomes_compile_error() {
        let got = expand_str(quote! { enum E { A } });
        assert_eq!(
            got,
            r#":: core :: compile_error ! { "Describe: только для структур, не для перечислений" }"#
        );
    }
    // ANCHOR_END: unit

    #[test]
    fn crate_path_from_attribute() {
        let got = expand_str(quote! {
            #[describe(crate = station)]
            struct P { x: u8 }
        });
        assert!(got.contains("impl station :: Describe for P"), "{got}");
        assert!(!got.contains("ch24"), "{got}");
    }

    // ANCHOR: outside
    /// А вот `proc_macro` вне вызова макроса компилятором не работает:
    /// его типы — ручки в таблицах, которые держит rustc.
    #[test]
    #[should_panic(expected = "procedural macro API is used outside of a procedural macro")]
    fn proc_macro_outside_of_macro_panics() {
        let _ = proc_macro::Span::call_site();
    }
    // ANCHOR_END: outside
}
