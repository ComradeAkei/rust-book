//! Задача D: `include_svd!("файл.svd", "UART0")` — API регистров одной
//! периферии из файла SVD, прочитанного при компиляции.
//!
//! Заготовка разбирает аргументы, читает файл, находит периферию и её
//! регистры с полями и порождает модуль со всеми нужными именами, но:
//! блок регистров без промежутков (смещения неверны), `OFFSET` и `RESET`
//! — нули, `read`, `write` и `modify` есть у всех регистров и паникуют,
//! `size`, `access` и `resetValue` не учтены, а любая ошибка в файле или
//! имени — паника макроса. Допишите (раздел 24.7).

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use roxmltree::{Document, Node};
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

/// Аргументы: путь к файлу (от каталога крейта, который вызывает макрос)
/// и имя периферии.
struct Args {
    path: LitStr,
    peripheral: LitStr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;
        input.parse::<Token![,]>()?;
        let peripheral = input.parse()?;
        input.parse::<Option<Token![,]>>()?;
        Ok(Args { path, peripheral })
    }
}

fn child<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    node.children().find(|c| c.has_tag_name(tag))
}

fn text<'a>(node: Node<'a, '_>, tag: &str) -> Option<&'a str> {
    child(node, tag).and_then(|c| c.text()).map(str::trim)
}

/// Число SVD: десятичное или `0x…`.
fn number(node: Node, tag: &str) -> u64 {
    let s = text(node, tag).unwrap_or_else(|| panic!("нет <{tag}>"));
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16).unwrap(),
        None => s.parse().unwrap(),
    }
}

/// `Dr`, `Ibrd`: имя типа регистра из имени в SVD.
fn type_name(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut cs = lower.chars();
    cs.next().map(|c| c.to_uppercase().chain(cs).collect()).unwrap_or_default()
}

// ANCHOR: skeleton
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let args: Args = syn::parse2(input)?;
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let xml = std::fs::read_to_string(std::path::Path::new(&dir).join(args.path.value())).unwrap();
    let doc = Document::parse(&xml).unwrap();
    let name = args.peripheral.value();
    let all = child(doc.root_element(), "peripherals").unwrap();
    let by_name = |n: &str| {
        all.children().find(|p| p.has_tag_name("peripheral") && text(*p, "name") == Some(n))
    };
    let p = by_name(&name).unwrap();
    let module = format_ident!("{}", name.to_lowercase());
    let base = Literal::u64_unsuffixed(number(p, "baseAddress"));
    // Регистры периферии с derivedFrom описаны у образца.
    let template = p.attribute("derivedFrom").map_or(p, |from| by_name(from).unwrap());

    let mut members = Vec::new();
    let mut regs = Vec::new();
    for r in child(template, "registers")
        .into_iter()
        .flat_map(|rs| rs.children())
        .filter(|c| c.has_tag_name("register"))
    {
        let rname = text(r, "name").unwrap();
        let field = format_ident!("{}", rname.to_lowercase());
        let ty = format_ident!("{}", type_name(rname));
        members.push(quote!(pub #field: #ty,));
        let fields = child(r, "fields")
            .into_iter()
            .flat_map(|fs| fs.children())
            .filter(|c| c.has_tag_name("field"))
            .map(|f| {
                let fname = format_ident!("{}", text(f, "name").unwrap().to_uppercase());
                let (o, w) = (number(f, "bitOffset") as u32, number(f, "bitWidth") as u32);
                quote!(pub const #fname: Field = Field { offset: #o, width: #w };)
            });
        regs.push(quote! {
            #[repr(transparent)]
            pub struct #ty(::core::cell::UnsafeCell<u32>);

            impl #ty {
                pub const OFFSET: usize = 0; // не решено
                pub const RESET: u32 = 0; // не решено
                #(#fields)*
                pub fn read(&self) -> u32 { panic!("не решено: read") }
                pub fn write(&self, value: u32) { let _ = value; panic!("не решено: write") }
                pub fn modify(&self, f: impl FnOnce(u32) -> u32) { let _ = f; panic!("не решено: modify") }
            }
        });
    }

    Ok(quote! {
        pub mod #module {
            pub const BASE: usize = #base;

            /// Битовое поле регистра.
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct Field {
                pub offset: u32,
                pub width: u32,
            }

            impl Field {
                pub const fn mask(self) -> u32 { 0 } // не решено
                pub const fn get(self, reg: u32) -> u32 { let _ = reg; 0 } // не решено
                pub const fn set(self, reg: u32, value: u32) -> u32 { let _ = value; reg } // не решено
            }

            /// Блок регистров — пока без промежутков между ними.
            #[repr(C)]
            pub struct RegisterBlock {
                #(#members)*
            }

            impl RegisterBlock {
                /// # Safety
                /// `ptr` выровнен на 4, указывает на `size_of::<RegisterBlock>()`
                /// байт, доступных на чтение и запись всё время `'a`.
                pub const unsafe fn from_ptr<'a>(ptr: *mut RegisterBlock) -> &'a RegisterBlock {
                    unsafe { &*ptr }
                }
            }

            /// # Safety
            /// Только на устройстве с этой периферией.
            pub unsafe fn steal() -> &'static RegisterBlock {
                unsafe { &*::core::ptr::with_exposed_provenance::<RegisterBlock>(BASE) }
            }

            #(#regs)*
        }
    })
}
// ANCHOR_END: skeleton
