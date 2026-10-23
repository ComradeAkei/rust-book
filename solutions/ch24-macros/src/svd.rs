//! Задача D: `include_svd!("файл.svd", "UART0")` — API регистров одной
//! периферии из файла SVD, прочитанного при компиляции.
//!
//! Поддержано подмножество CMSIS-SVD, которого хватает описанию UART
//! lm3s6965: периферии с `derivedFrom`, регистры с `addressOffset`, `size`,
//! `access`, `resetValue` (с наследованием значений по умолчанию от
//! устройства и периферии) и поля с `bitOffset` и `bitWidth`.

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use roxmltree::{Document, Node};
use syn::parse::{Parse, ParseStream};
use syn::{Error, LitStr, Token};

// ANCHOR: args
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
// ANCHOR_END: args

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Access {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Clone, Copy)]
struct Defaults {
    size: u32,
    access: Access,
    reset: u64,
}

#[derive(Clone, Debug)]
struct Field {
    name: String,
    description: Option<String>,
    offset: u32,
    width: u32,
}

#[derive(Clone, Debug)]
struct Register {
    name: String,
    description: Option<String>,
    offset: u32,
    size: u32,
    access: Access,
    reset: u64,
    fields: Vec<Field>,
}

struct Peripheral {
    name: String,
    description: Option<String>,
    base: u64,
    registers: Vec<Register>,
}

/// Ошибка в файле: текст и где её показать.
struct SvdError {
    msg: String,
    place: Place,
}

enum Place {
    /// Строка и столбец в файле.
    At(u32, u32),
    /// Файл в целом.
    File,
    /// Имя периферии в вызове макроса.
    Name,
}

type SvdResult<T> = Result<T, SvdError>;

fn at(node: Node, msg: impl Into<String>) -> SvdError {
    let p = node.document().text_pos_at(node.range().start);
    SvdError { msg: msg.into(), place: Place::At(p.row, p.col) }
}

fn child<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    node.children().find(|c| c.has_tag_name(tag))
}

fn text<'a>(node: Node<'a, '_>, tag: &str) -> Option<&'a str> {
    child(node, tag).and_then(|c| c.text()).map(str::trim)
}

fn required<'a>(node: Node<'a, '_>, tag: &str, what: &str) -> SvdResult<&'a str> {
    text(node, tag).ok_or_else(|| at(node, format!("{what}: нет <{tag}>")))
}

// ANCHOR: number
/// Число SVD: десятичное, `0x…` или `#…` (двоичное).
fn number(node: Node, tag: &str, what: &str) -> SvdResult<Option<u64>> {
    let Some(s) = text(node, tag) else { return Ok(None) };
    let parsed = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
    } else if let Some(bin) = s.strip_prefix('#') {
        u64::from_str_radix(bin, 2)
    } else {
        s.parse()
    };
    parsed.map(Some).map_err(|_| {
        at(child(node, tag).unwrap(), format!("{what}: <{tag}>{s}</{tag}> — не число"))
    })
}
// ANCHOR_END: number

fn access(node: Node, what: &str, default: Access) -> SvdResult<Access> {
    Ok(match text(node, "access") {
        None => default,
        Some("read-only") => Access::ReadOnly,
        Some("write-only") => Access::WriteOnly,
        Some("read-write") => Access::ReadWrite,
        Some(other) => {
            return Err(at(
                child(node, "access").unwrap(),
                format!("{what}: доступ `{other}` не поддержан"),
            ));
        }
    })
}

fn defaults(node: Node, inherited: Defaults, what: &str) -> SvdResult<Defaults> {
    Ok(Defaults {
        size: number(node, "size", what)?.map_or(inherited.size, |v| v as u32),
        access: access(node, what, inherited.access)?,
        reset: number(node, "resetValue", what)?.unwrap_or(inherited.reset),
    })
}

fn field(node: Node, reg: &str, reg_size: u32) -> SvdResult<Field> {
    let name = required(node, "name", reg)?.to_string();
    let what = format!("поле {reg}.{name}");
    let offset = number(node, "bitOffset", &what)?
        .ok_or_else(|| at(node, format!("{what}: нет <bitOffset>")))? as u32;
    let width = number(node, "bitWidth", &what)?
        .ok_or_else(|| at(node, format!("{what}: нет <bitWidth>")))? as u32;
    if width == 0 || offset + width > reg_size {
        return Err(at(
            node,
            format!(
                "{what}: биты {offset}..{} не помещаются в {reg_size}-битный регистр",
                offset + width
            ),
        ));
    }
    Ok(Field { name, description: text(node, "description").map(str::to_string), offset, width })
}

fn register(node: Node, inherited: Defaults) -> SvdResult<Register> {
    let name = required(node, "name", "регистр")?.to_string();
    let what = format!("регистр {name}");
    let d = defaults(node, inherited, &what)?;
    if ![8, 16, 32].contains(&d.size) {
        return Err(at(node, format!("{what}: размер {} бит не поддержан", d.size)));
    }
    let offset = number(node, "addressOffset", &what)?
        .ok_or_else(|| at(node, format!("{what}: нет <addressOffset>")))? as u32;
    if offset % (d.size / 8) != 0 {
        return Err(at(node, format!("{what}: смещение {offset:#x} не кратно размеру регистра")));
    }
    let mut fields = Vec::new();
    if let Some(fs) = child(node, "fields") {
        for f in fs.children().filter(|c| c.has_tag_name("field")) {
            fields.push(field(f, &name, d.size)?);
        }
    }
    for (i, a) in fields.iter().enumerate() {
        for b in &fields[..i] {
            if a.offset < b.offset + b.width && b.offset < a.offset + a.width {
                return Err(at(
                    node,
                    format!("{what}: поля {} и {} перекрываются", b.name, a.name),
                ));
            }
        }
    }
    Ok(Register {
        name,
        description: text(node, "description").map(str::to_string),
        offset,
        size: d.size,
        access: d.access,
        reset: d.reset,
        fields,
    })
}

fn registers(p: Node, inherited: Defaults) -> SvdResult<Vec<Register>> {
    let mut regs = Vec::new();
    if let Some(rs) = child(p, "registers") {
        for r in rs.children().filter(|c| c.has_tag_name("register")) {
            regs.push(register(r, inherited)?);
        }
    }
    regs.sort_by_key(|r| r.offset);
    for w in regs.windows(2) {
        if w[0].offset + w[0].size / 8 > w[1].offset {
            return Err(SvdError {
                msg: format!("регистры {} и {} перекрываются", w[0].name, w[1].name),
                place: Place::File,
            });
        }
    }
    Ok(regs)
}

// ANCHOR: find
/// Периферия по имени; у `derivedFrom` регистры берутся у образца.
fn find(doc: &Document, name: &str) -> SvdResult<Peripheral> {
    let device = doc.root_element();
    if !device.has_tag_name("device") {
        return Err(at(device, "корневой элемент — не <device>"));
    }
    let dev_defaults =
        defaults(device, Defaults { size: 32, access: Access::ReadWrite, reset: 0 }, "устройство")?;
    let all: Vec<Node> = child(device, "peripherals")
        .map(|ps| ps.children().filter(|c| c.has_tag_name("peripheral")).collect())
        .unwrap_or_default();
    let by_name = |n: &str| all.iter().copied().find(|p| text(*p, "name") == Some(n));
    let Some(p) = by_name(name) else {
        let names: Vec<_> = all.iter().filter_map(|p| text(*p, "name")).collect();
        return Err(SvdError {
            msg: format!("нет периферии {name}; есть: {}", names.join(", ")),
            place: Place::Name,
        });
    };
    let what = format!("периферия {name}");
    let base = number(p, "baseAddress", &what)?
        .ok_or_else(|| at(p, format!("{what}: нет <baseAddress>")))?;
    let (template, description) = match p.attribute("derivedFrom") {
        Some(from) => {
            let t = by_name(from).ok_or_else(|| {
                at(p, format!("{what}: derivedFrom=\"{from}\" — такой периферии нет"))
            })?;
            (t, text(p, "description").or_else(|| text(t, "description")))
        }
        None => (p, text(p, "description")),
    };
    let inherited = defaults(template, dev_defaults, &what)?;
    Ok(Peripheral {
        name: name.to_string(),
        description: description.map(str::to_string),
        base,
        registers: registers(template, inherited)?,
    })
}
// ANCHOR_END: find

/// `Dr`, `Ibrd`: имя типа регистра из имени в SVD.
fn type_name(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut cs = lower.chars();
    cs.next().map(|c| c.to_uppercase().chain(cs).collect()).unwrap_or_default()
}

fn hex(v: u64) -> Literal {
    Literal::u64_unsuffixed(v)
}

// ANCHOR: codegen
fn codegen(p: &Peripheral, file: &str) -> TokenStream {
    let module = format_ident!("{}", p.name.to_lowercase());
    let base = Literal::usize_unsuffixed(p.base as usize);
    let doc = p.description.clone().unwrap_or_else(|| p.name.clone());
    let end = p.registers.last().map_or(0, |r| r.offset + r.size / 8);
    let size = Literal::usize_unsuffixed(end as usize);

    let mut members = Vec::new();
    let mut checks = Vec::new();
    let mut at = 0u32;
    for (i, r) in p.registers.iter().enumerate() {
        if r.offset > at {
            let pad = format_ident!("_reserved{}", i);
            let n = Literal::usize_unsuffixed((r.offset - at) as usize);
            members.push(quote!(#pad: [::core::primitive::u8; #n],));
        }
        let field = format_ident!("{}", r.name.to_lowercase());
        let ty = format_ident!("{}", type_name(&r.name));
        let off = Literal::usize_unsuffixed(r.offset as usize);
        let doc = r.description.clone().unwrap_or_default();
        members.push(quote!(#[doc = #doc] pub #field: #ty,));
        checks
            .push(quote!(::core::assert!(::core::mem::offset_of!(RegisterBlock, #field) == #off);));
        at = r.offset + r.size / 8;
    }

    let regs = p.registers.iter().map(|r| {
        let ty = format_ident!("{}", type_name(&r.name));
        let word = format_ident!("u{}", r.size);
        let off = Literal::usize_unsuffixed(r.offset as usize);
        let reset = hex(r.reset);
        let doc = r.description.clone().unwrap_or_else(|| r.name.clone());
        let fields = r.fields.iter().map(|f| {
            let name = format_ident!("{}", f.name.to_uppercase());
            let (o, w) = (f.offset, f.width);
            let doc = f.description.clone().unwrap_or_else(|| f.name.clone());
            quote!(#[doc = #doc] pub const #name: Field = Field { offset: #o, width: #w };)
        });
        let read = (r.access != Access::WriteOnly).then(|| {
            quote! {
                #[inline]
                pub fn read(&self) -> ::core::primitive::#word {
                    // SAFETY: регистр устройства или память блока, отданная
                    // через `steal` или `from_ptr`; доступ volatile.
                    unsafe { ::core::ptr::read_volatile(self.0.get()) }
                }
            }
        });
        let write = (r.access != Access::ReadOnly).then(|| {
            quote! {
                #[inline]
                pub fn write(&self, value: ::core::primitive::#word) {
                    // SAFETY: см. read.
                    unsafe { ::core::ptr::write_volatile(self.0.get(), value) }
                }
            }
        });
        let modify = (r.access == Access::ReadWrite).then(|| {
            quote! {
                #[inline]
                pub fn modify(&self, f: impl ::core::ops::FnOnce(::core::primitive::#word) -> ::core::primitive::#word) {
                    self.write(f(self.read()))
                }
            }
        });
        quote! {
            #[doc = #doc]
            #[repr(transparent)]
            pub struct #ty(::core::cell::UnsafeCell<::core::primitive::#word>);

            impl #ty {
                pub const OFFSET: ::core::primitive::usize = #off;
                pub const RESET: ::core::primitive::#word = #reset;
                #(#fields)*
                #read
                #write
                #modify
            }
        }
    });

    // Файл — зависимость сборки: include_bytes! попадает в dep-info, и
    // cargo пересоберёт крейт, когда SVD изменится.
    quote! {
        #[doc = #doc]
        pub mod #module {
            /// Адрес блока регистров.
            pub const BASE: ::core::primitive::usize = #base;

            /// Битовое поле регистра.
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            pub struct Field {
                pub offset: ::core::primitive::u32,
                pub width: ::core::primitive::u32,
            }

            impl Field {
                pub const fn mask(self) -> ::core::primitive::u32 {
                    (::core::primitive::u32::MAX >> (32 - self.width)) << self.offset
                }
                pub const fn get(self, reg: ::core::primitive::u32) -> ::core::primitive::u32 {
                    (reg & self.mask()) >> self.offset
                }
                pub const fn set(self, reg: ::core::primitive::u32, value: ::core::primitive::u32) -> ::core::primitive::u32 {
                    (reg & !self.mask()) | ((value << self.offset) & self.mask())
                }
            }

            /// Блок регистров в порядке адресов, с промежутками.
            #[repr(C)]
            pub struct RegisterBlock {
                #(#members)*
            }

            impl RegisterBlock {
                /// Блок поверх памяти по указателю (для тестов на хосте).
                ///
                /// # Safety
                /// `ptr` выровнен на 4, указывает на `size_of::<RegisterBlock>()`
                /// байт, доступных на чтение и запись всё время `'a`.
                pub const unsafe fn from_ptr<'a>(ptr: *mut RegisterBlock) -> &'a RegisterBlock {
                    unsafe { &*ptr }
                }
            }

            /// Регистры устройства по адресу `BASE`.
            ///
            /// # Safety
            /// Только на устройстве с этой периферией; кто ещё держит
            /// ссылку на регистры, решает вызывающий.
            pub unsafe fn steal() -> &'static RegisterBlock {
                unsafe { &*::core::ptr::with_exposed_provenance::<RegisterBlock>(BASE) }
            }

            #(#regs)*

            const _: () = {
                ::core::assert!(::core::mem::size_of::<RegisterBlock>() == #size);
                #(#checks)*
            };
            const _: &[::core::primitive::u8] = ::core::include_bytes!(#file);
        }
    }
}
// ANCHOR_END: codegen

// ANCHOR: expand
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let args: Args = syn::parse2(input)?;
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let path = std::path::Path::new(&dir).join(args.path.value());
    let shown = args.path.value();
    let xml = std::fs::read_to_string(&path)
        .map_err(|e| Error::new_spanned(&args.path, format!("{shown}: {e}")))?;
    let doc = Document::parse(&xml)
        .map_err(|e| Error::new_spanned(&args.path, format!("{shown}: {e}")))?;
    // Ошибка в файле — на литерале пути, нет такой периферии — на её имени.
    let peripheral = find(&doc, &args.peripheral.value()).map_err(|e| match e.place {
        Place::At(row, col) => {
            Error::new_spanned(&args.path, format!("{shown}:{row}:{col}: {}", e.msg))
        }
        Place::File => Error::new_spanned(&args.path, format!("{shown}: {}", e.msg)),
        Place::Name => Error::new_spanned(&args.peripheral, format!("{shown}: {}", e.msg)),
    })?;
    let file = path.canonicalize().unwrap_or(path).display().to_string();
    Ok(codegen(&peripheral, &file))
}
// ANCHOR_END: expand
