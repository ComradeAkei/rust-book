//! Задача B: `#[derive(Bitfield)]` — упаковка полей с `#[bits(N)]` в `u32`.
//!
//! Заготовка разбирает именованные поля и их `#[bits(N)]` и порождает тип
//! `{Имя}Bits(pub u32)` со всеми методами и константами, но с заглушками:
//! сдвиги и маски — нули, геттеры возвращают `false` и `0`, `set_*` и
//! `with_*` ничего не меняют. Допишите расчёт раскладки, тела методов,
//! `Debug`, преобразования и ошибки со спанами (раздел 24.7).

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Fields, Ident, LitInt, Type};

struct Packed<'a> {
    name: &'a Ident,
    ty: &'a Type,
    is_bool: bool,
    #[allow(dead_code)] // понадобится для раскладки
    width: u32,
}

// ANCHOR: skeleton
fn fields(input: &DeriveInput) -> syn::Result<Vec<Packed<'_>>> {
    let Data::Struct(s) = &input.data else {
        return Err(Error::new_spanned(&input.ident, "не решено: Bitfield только для структур"));
    };
    let Fields::Named(named) = &s.fields else {
        return Err(Error::new_spanned(&input.ident, "не решено: нужны именованные поля"));
    };
    let mut out = Vec::new();
    for f in &named.named {
        let mut width = 0;
        for attr in f.attrs.iter().filter(|a| a.path().is_ident("bits")) {
            width = attr.parse_args::<LitInt>()?.base10_parse()?;
        }
        let is_bool = matches!(&f.ty, Type::Path(p) if p.path.is_ident("bool"));
        out.push(Packed { name: f.ident.as_ref().unwrap(), ty: &f.ty, is_bool, width });
    }
    Ok(out)
}

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let fields = fields(&input)?;
    let vis = &input.vis;
    let name = &input.ident;
    let bits = format_ident!("{}Bits", name);
    let methods = fields.iter().map(|f| {
        let (fname, ty) = (f.name, f.ty);
        let shift = format_ident!("{}_SHIFT", fname.to_string().to_uppercase());
        let mask = format_ident!("{}_MASK", fname.to_string().to_uppercase());
        let setter = format_ident!("set_{}", fname);
        let with = format_ident!("with_{}", fname);
        let zero = if f.is_bool { quote!(false) } else { quote!(0) };
        quote! {
            pub const #shift: u32 = 0; // не решено
            pub const #mask: u32 = 0; // не решено
            pub const fn #fname(self) -> #ty { #zero } // не решено
            pub const fn #setter(&mut self, value: #ty) { let _ = value; } // не решено
            pub const fn #with(self, value: #ty) -> Self { let _ = value; self } // не решено
        }
    });
    let names: Vec<_> = fields.iter().map(|f| f.name).collect();
    Ok(quote! {
        #[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
        #[repr(transparent)]
        #vis struct #bits(pub u32);

        impl #bits {
            pub const WIDTH: u32 = 0; // не решено
            #(#methods)*
        }

        impl ::core::convert::From<#name> for #bits {
            fn from(_: #name) -> Self { Self(0) } // не решено
        }

        impl ::core::convert::From<#bits> for #name {
            fn from(b: #bits) -> Self { Self { #(#names: b.#names()),* } } // не решено
        }
    })
}
// ANCHOR_END: skeleton

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match syn::parse::<DeriveInput>(input) {
        Ok(i) => i,
        Err(e) => return e.into_compile_error().into(),
    };
    expand(input).unwrap_or_else(Error::into_compile_error).into()
}
