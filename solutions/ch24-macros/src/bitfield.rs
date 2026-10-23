//! Задача B: `#[derive(Bitfield)]` — упаковка полей с `#[bits(N)]` в `u32`.
//!
//! Derive не может изменить структуру пользователя, поэтому рядом с ней
//! появляется тип `{Имя}Bits(pub u32)` — то самое слово регистра — с
//! геттерами, `set_*`, `with_*`, константами сдвигов и масок и
//! преобразованиями в обе стороны.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Data, DeriveInput, Error, Fields, Ident, LitInt, Type};

fn push(errors: &mut Option<Error>, e: Error) {
    match errors {
        Some(all) => all.combine(e),
        None => *errors = Some(e),
    }
}

/// Тип поля и его ширина в битах.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bool,
    Uint(u32),
}

fn kind(ty: &Type) -> Option<Kind> {
    let Type::Path(p) = ty else { return None };
    if p.qself.is_some() {
        return None;
    }
    let id = p.path.get_ident()?;
    Some(match id.to_string().as_str() {
        "bool" => Kind::Bool,
        "u8" => Kind::Uint(8),
        "u16" => Kind::Uint(16),
        "u32" => Kind::Uint(32),
        _ => return None,
    })
}

struct Packed<'a> {
    name: &'a Ident,
    ty: &'a Type,
    kind: Kind,
    offset: u32,
    width: u32,
}

/// Ширина из `#[bits(N)]`: литерал вместе со значением, чтобы ошибки
/// указывали на него.
fn bits_attr(field: &syn::Field) -> syn::Result<Option<(LitInt, u32)>> {
    let mut found = None;
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("bits")) {
        if found.is_some() {
            return Err(Error::new_spanned(attr, "второй #[bits] у одного поля"));
        }
        let lit: LitInt = attr.parse_args()?;
        let n: u32 = lit.base10_parse()?;
        found = Some((lit, n));
    }
    Ok(found)
}

// ANCHOR: layout
fn layout(input: &DeriveInput) -> syn::Result<Vec<Packed<'_>>> {
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            other => return Err(Error::new_spanned(other, "Bitfield: нужны именованные поля")),
        },
        Data::Enum(e) => {
            return Err(Error::new_spanned(e.enum_token, "Bitfield: только для структур"));
        }
        Data::Union(u) => {
            return Err(Error::new_spanned(u.union_token, "Bitfield: только для структур"));
        }
    };
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            "Bitfield: у структуры не может быть параметров",
        ));
    }
    let mut errors = None;
    let mut packed = Vec::new();
    let mut offset = 0u32;
    for f in fields {
        let name = f.ident.as_ref().unwrap();
        let attr = match bits_attr(f) {
            Ok(Some(a)) => a,
            Ok(None) => {
                push(
                    &mut errors,
                    Error::new_spanned(name, format!("поле `{name}` без #[bits(N)]")),
                );
                continue;
            }
            Err(e) => {
                push(&mut errors, e);
                continue;
            }
        };
        let (lit, width) = attr;
        let Some(kind) = kind(&f.ty) else {
            push(
                &mut errors,
                Error::new_spanned(&f.ty, "Bitfield: тип поля — bool, u8, u16 или u32"),
            );
            continue;
        };
        let fits = match kind {
            Kind::Bool => width == 1,
            Kind::Uint(max) => (1..=max).contains(&width),
        };
        if !fits {
            let msg = match kind {
                Kind::Bool => "поле bool занимает ровно 1 бит".to_string(),
                Kind::Uint(_) if width == 0 => "ширина поля — от 1 бита".to_string(),
                Kind::Uint(max) => format!("{width} бит не помещаются в u{max}"),
            };
            push(&mut errors, Error::new_spanned(&lit, msg));
            continue;
        }
        if offset + width > 32 {
            let msg =
                format!("поле `{name}` занимает биты {offset}..{}, а в u32 их 32", offset + width);
            push(&mut errors, Error::new_spanned(&lit, msg));
        }
        packed.push(Packed { name, ty: &f.ty, kind, offset, width });
        offset += width;
    }
    errors.map_or(Ok(packed), Err)
}
// ANCHOR_END: layout

fn upper(name: &Ident) -> String {
    name.to_string().to_uppercase()
}

// ANCHOR: expand
pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let fields = layout(&input)?;
    let vis = &input.vis;
    let name = &input.ident;
    let bits = format_ident!("{}Bits", name);
    let total: u32 = fields.iter().map(|f| f.width).sum();

    let methods = fields.iter().map(|f| {
        let (fname, ty, off, width) = (f.name, f.ty, f.offset, f.width);
        let shift = format_ident!("{}_SHIFT", upper(fname));
        let mask = format_ident!("{}_MASK", upper(fname));
        let setter = format_ident!("set_{}", fname);
        let with = format_ident!("with_{}", fname);
        let mask_val = if width == 32 { u32::MAX } else { (1u32 << width) - 1 };
        let mask_lit = proc_macro2::Literal::u32_suffixed(mask_val);
        // Методы получают спан имени поля: ошибка «нет метода» или
        // конфликт имён укажет на поле пользователя.
        let (get, put) = match f.kind {
            Kind::Bool => (quote!(raw != 0), quote!(value as ::core::primitive::u32)),
            Kind::Uint(_) => (quote!(raw as #ty), quote!(value as ::core::primitive::u32)),
        };
        let overflow = format!("{name}.{fname}: значение не помещается в {width} бит");
        quote_spanned! {fname.span()=>
            pub const #shift: ::core::primitive::u32 = #off;
            pub const #mask: ::core::primitive::u32 = #mask_lit << #off;

            pub const fn #fname(self) -> #ty {
                let raw = (self.0 >> #off) & #mask_lit;
                #get
            }

            pub const fn #setter(&mut self, value: #ty) {
                let value = #put;
                ::core::assert!(value <= #mask_lit, #overflow);
                self.0 = (self.0 & !Self::#mask) | (value << #off);
            }

            pub const fn #with(mut self, value: #ty) -> Self {
                self.#setter(value);
                self
            }
        }
    });
    let names: Vec<_> = fields.iter().map(|f| f.name).collect();
    let setters: Vec<_> = fields.iter().map(|f| format_ident!("set_{}", f.name)).collect();
    let bits_doc = format!("Слово регистра для [`{name}`]: {total} бит из 32.");

    Ok(quote! {
        #[doc = #bits_doc]
        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq, ::core::default::Default)]
        #[repr(transparent)]
        #vis struct #bits(pub ::core::primitive::u32);

        impl #bits {
            /// Сколько бит занимают поля.
            pub const WIDTH: ::core::primitive::u32 = #total;
            #(#methods)*
        }

        impl ::core::convert::From<#name> for #bits {
            fn from(v: #name) -> Self {
                let mut b = Self(0);
                #(b.#setters(v.#names);)*
                b
            }
        }

        impl ::core::convert::From<#bits> for #name {
            fn from(b: #bits) -> Self {
                Self { #(#names: b.#names()),* }
            }
        }

        impl ::core::fmt::Debug for #bits {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(::core::stringify!(#bits))
                    #(.field(::core::stringify!(#names), &self.#names()))*
                    .finish()
            }
        }
    })
}
// ANCHOR_END: expand

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match syn::parse::<DeriveInput>(input) {
        Ok(i) => i,
        Err(e) => return e.into_compile_error().into(),
    };
    expand(input).unwrap_or_else(Error::into_compile_error).into()
}
