//! Процедурные макросы главы 24. Пользоваться ими нужно через крейт `ch24`:
//! он реэкспортирует макросы и даёт трейты и функции, на которые ссылается
//! сгенерированный код.
//!
//! - [`token_tree!`] — 24.1: дерево токенов на одном API `proc_macro`.
//! - [`Describe`] — 24.2–24.4: derive на syn и quote с атрибутами-помощниками;
//!   [`DescribeCallSite`] — тот же derive с плохими спанами (24.3).
//! - [`traced`] и [`traced_call_site`] — 24.3: гигиена имён в атрибуте.
//! - [`interrupt`] — 24.4: обработчик прерывания с проверкой сигнатуры.
//!
//! Корень крейта процедурных макросов может экспортировать только сами
//! макросы, поэтому логика лежит в закрытых модулях и написана на
//! proc_macro2: её проверяют модульные тесты (24.5).

mod describe;
mod interrupt;
mod tokens;
mod traced;

use proc_macro::TokenStream;
use proc_macro2::Span;

// ANCHOR: entry_points
/// Функциональный макрос: `token_tree!(...)` раскрывается в строковый
/// литерал с описанием каждого токена входа.
#[proc_macro]
pub fn token_tree(input: TokenStream) -> TokenStream {
    tokens::token_tree(input)
}

/// Derive: добавляет `impl ch24::Describe` рядом со структурой. Саму
/// структуру derive изменить не может.
#[proc_macro_derive(Describe, attributes(describe))]
pub fn derive_describe(input: TokenStream) -> TokenStream {
    describe::derive(input, describe::Spans::User)
}

/// Атрибут: получает аргументы атрибута и весь элемент, под которым стоит,
/// и заменяет элемент своим выходом.
#[proc_macro_attribute]
pub fn interrupt(attr: TokenStream, item: TokenStream) -> TokenStream {
    match interrupt::expand(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
// ANCHOR_END: entry_points

/// Тот же derive, что [`Describe`], но весь сгенерированный код получает
/// спан места вызова — образец того, как не надо (раздел 24.3).
#[proc_macro_derive(DescribeCallSite, attributes(describe))]
pub fn derive_describe_call_site(input: TokenStream) -> TokenStream {
    describe::derive(input, describe::Spans::CallSite)
}

/// `#[traced]`: переменная макроса с гигиеной `mixed_site` (24.3).
#[proc_macro_attribute]
pub fn traced(attr: TokenStream, item: TokenStream) -> TokenStream {
    match traced::expand(attr.into(), item.into(), Span::mixed_site()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

/// `#[traced_call_site]`: та же переменная со спаном места вызова —
/// захватывает одноимённый аргумент функции (24.3).
#[proc_macro_attribute]
pub fn traced_call_site(attr: TokenStream, item: TokenStream) -> TokenStream {
    match traced::expand(attr.into(), item.into(), Span::call_site()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
