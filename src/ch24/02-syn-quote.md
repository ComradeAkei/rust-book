# 24.2. syn и quote: derive

## Описание раскладки из объявления

В главе 6 [раскладку][g-layout] структуры смотрели тестами: `size_of`, `align_of`,
`offset_of!` для каждого поля. Для протокола или регистровой карты такие
проверки хочется иметь всегда — и печать кадра с именами полей заодно.
Руками это таблица, которая разойдётся с объявлением при первой же
правке. Derive строит её из самого объявления. Трейт, который он
реализует:

```rust,ignore
{{#include ../../code/ch24/src/describe.rs:trait}}
```

И заголовок кадра телеметрии с ним:

```rust,ignore
{{#include ../../code/ch24/src/describe.rs:header}}
```

```rust,ignore
{{#include ../../code/ch24/src/describe.rs:header_test}}
```

Функция `layout` из `ch24` идёт по `FIELDS` в порядке смещений и видит
дыру выравнивания между `kind` и `crc`; `dump` печатает значения через
`Debug`. Атрибут `#[describe(size = 8)]` превращается в проверку при
компиляции — о нём в разделе [24.3](03-spans.md).

## Разбор: syn

Вход derive — токены объявления `pub struct Header { pub magic: u16, ...
}` вместе с атрибутами. Разбирать их руками — значит написать парсер
Rust. Его уже написали: крейт **syn** разбирает поток токенов в
синтаксическое дерево — `DeriveInput` для derive, `ItemFn` для функции,
`Expr`, `Type`, `Path` — всего больше двухсот типов. Дерево сохраняет
исходные токены и их спаны: любой узел можно вернуть в поток (`ToTokens`)
и на любой указать ошибкой.

```rust,ignore
pub fn derive(input: proc_macro::TokenStream, spans: Spans) -> proc_macro::TokenStream {
    match syn::parse::<DeriveInput>(input) {
        Ok(input) => expand(input, spans).unwrap_or_else(Error::into_compile_error).into(),
        Err(e) => e.into_compile_error().into(),
    }
}
```

`DeriveInput` — это `attrs` (все атрибуты, включая `#[repr(C)]` и
`#[describe]`), `vis`, `ident`, `generics` и `data`: `Struct` с полями,
`Enum` с вариантами или `Union`. Ошибка разбора и ошибка самого макроса
превращаются в токены `compile_error!` — об этом в разделе
[24.3](03-spans.md). Возможности syn включают только нужное: `derive`
хватает для `DeriveInput`, а для атрибутов на функциях нужен `full` с
разбором выражений и операторов ([24.6](06-cost.md)).

## Сборка: quote!

Выход собирает макрос **quote!** — шаблон, который возвращает
`proc_macro2::TokenStream`. Внутри — обычный код Rust, а `#имя`
подставляет значение переменной (любого типа с `ToTokens`: `Ident`,
`Type`, `LitStr`, другой `TokenStream`); `#(#entries),*` — повторение,
как `$(...),*` в [`macro_rules!`][g-declarative-macro]. Весь derive:

```rust,ignore
{{#include ../../code/ch24-macros/src/describe.rs:expand}}
```

По шагам:

- **отказ от перечислений и объединений** — ошибка со спаном ключевого
  слова `enum` или `union` (раздел [24.3](03-spans.md));
- **поля**: у именованного поля член — `Member::Named(magic)`, у
  кортежного — `Member::Unnamed(0)`; `offset_of!(Self, 0)` и `self.0`
  работают с обоими;
- **имя крейта** — `::ch24` или путь из атрибута (ниже);
- **обобщения** — `split_for_impl` (ниже);
- **поля в константе**: для каждого — `Field { name, ty, offset, size }`;
  текст типа даёт `::core::stringify!(#ty)`, смещение —
  `::core::mem::offset_of!`, размер — `size_of`;
- **дамп** — по вызову `dump_field` на поле; `&self.поле` приводится к
  `&dyn Debug`, и поле без `Debug` даст ошибку E0277 (о том, *куда* она
  укажет, — раздел [24.3](03-spans.md)).

Код не строится строкой. Соблазн велик — `format!("impl Describe for
{name} {{ ... }}")` и `parse()`, — но такой код теряет спаны: токены,
разобранные из строки, «написаны» в месте вызова, и любая ошибка в них
укажет на `#[derive(...)]`. `quote!` переносит спаны подставленных узлов:
`#ty` в выходе — те самые токены типа из объявления пользователя.

Атрибут `#[automatically_derived]` на `impl` — пометка «сгенерировано»:
по ней [линты][g-lint] и инструменты отличают код derive от рукописного. Так
помечает свои реализации и `#[derive(Debug)]` из std (задача A).

## Обобщения

Для `struct Sample<T>` нужен `impl<T> Describe for Sample<T> where T:
Debug`, и собирать эти три части руками — значит разбирать параметры,
времена жизни, константные параметры и `where`. `split_for_impl` делит
`Generics` на «параметры impl» (с ограничениями), «аргументы типа» (без
них) и `where`. Ограничение `T: Debug` добавлено к каждому параметру
типа — как это делает `#[derive(Debug)]` из std, — иначе `dump` не смог
бы напечатать поле типа `T`.

```rust,ignore
{{#include ../../code/ch24/src/describe.rs:generic}}
```

Тест `generic_and_tuple`: у `Sample<u64>` поле `value` лежит со
смещением 8 и размером 8, у `Sample<u16>` — со смещением 4 и размером 2,
хотя код один. Константа `FIELDS` вычисляется для каждой подстановки
отдельно — `offset_of!(Self, value)` знает тип. А вот `ty` у обоих —
`"T"`: макрос раскрывается *до* проверки типов и видит только текст
объявления. Что такое `T` в конкретной подстановке, он не знает и знать
не может.

## Полные пути

Сгенерированный `impl` раскрывается в [модуле][g-module] пользователя, и пути в нём
разрешаются там же — та же история, что в разделе
[23.3](../ch23/03-hygiene.md), только у процедурного макроса нет даже
`$crate`. Модуль-провокация:

```rust,ignore
{{#include ../../code/ch24/src/describe.rs:hostile}}
```

Здесь свои `core`, `fmt` и `ch24`, свои `Option` и `Result` — и свой тип
`usize`. Последнее неожиданно: имена примитивных типов в Rust — не
ключевые слова, а имена в [прелюдии][g-prelude] языка, и объявленный в модуле `struct
usize;` их перекрывает. Сгенерированное `const SIZE: usize` в этом модуле
стало бы константой типа `hostile::usize`. Поэтому в выходе derive всё
от корня: `::core::mem::size_of`, `::core::result::Result::Ok`,
`::core::fmt::Write` и `::core::primitive::usize` для самих примитивов.
`::core` есть и в `no_std`, так что derive годится для прошивки.

## Свой крейт без `$crate`

Трейт `Describe` и функции `dump_field` живут в `ch24`, и сгенерированный
код ссылается на `::ch24::Describe`. Внутри самого `ch24` такого пути
нет: изнутри крейт называет себя `crate`, а не `::ch24`. Примеры
`#[derive(Describe)]` в `ch24` компилируются благодаря одной строке:

```rust,ignore
{{#include ../../code/ch24/src/lib.rs:self_alias}}
```

`extern crate self as ch24;` объявляет в корне крейта имя `ch24` для
самого себя, и путь `::ch24::Describe` разрешается и снаружи, и внутри.
Третий случай — пользователь переименовал зависимость в `Cargo.toml`, и
`::ch24` у него нет вовсе. Тогда путь к крейту называет он сам атрибутом
`#[describe(crate = ...)]` (раздел [24.4](04-attributes.md)) — так же
поступают serde (`#[serde(crate = "...")]`) и многие другие.

## Что получилось

`rustc -Zunpretty=expanded` для `Header` (тест
`rustc_expansions_match_snapshots` сверяет снимок с закреплённым rustc):

```rust,ignore
{{#include ../../code/ch24/expanded/describe_header.rs}}
```

Печать раскрытия идёт после раскрытия *всех* макросов, поэтому
`stringify!(u16)` уже стал строкой `"u16"`, `offset_of!` — встроенной
формой `builtin # offset_of`, а `assert!` из проверки размера — `if` с
вызовом `panic_fmt`. Последний элемент, `const _: () = ...`, —
безымянная константа: её вычисляет компилятор, и ошибка вычисления —
ошибка сборки ([24.3](03-spans.md)).

<div class="box trap">

**Ловушка — `stringify!` от чужих токенов.** Текст типа у `Header` —
ровно `"u16"`, но для типов посложнее rustc 1.97 печатает токены,
прошедшие через процедурный макрос, с пробелами между ними: `&'a [u16]`
становится `"& 'a [u16]"`, а `Option<&'a Buf<'a>>` — `"Option < & 'a Buf
< 'a > >"` (тест `observed_stringify_spacing`). Вёрстка `stringify!` не
гарантирована и менялась между версиями. Сравнивать такой текст можно
только без пробелов (тест `type_text_comes_from_stringify`), а
показывать — как есть.

</div>

<div class="box c">

**Как в C.** Ближайший аналог — X-macro: список полей пишется один раз,
и из него раскрываются и структура, и таблица `offsetof`. Но объявление
структуры тогда тоже приходится писать через макрос, а не как обычную
структуру C. Derive работает с обычным объявлением: структура остаётся
структурой, и её можно читать, документировать и править как любую
другую.

</div>

<div class="box check">

**Проверка.** Тесты `header_fields`, `header_layout_and_dump`,
`generic_and_tuple`, `hostile_module` и `type_text_comes_from_stringify`
в `code/ch24/src/describe.rs`; `repr_c_keeps_declaration_order` и
наблюдение `observed_rust_repr_reorders` — раскладка `repr(Rust)` у
rustc 1.97 переставляет поля (8 байт без дыр вместо 12 у `repr(C)`);
модульные `expands_to_impl` и `crate_path_from_attribute` в
`code/ch24-macros` сравнивают выход derive с ожидаемым `quote!`.

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-declarative-macro]: ../appendix-f.md#declarative-macro "Декларативный макрос (declarative macro): Макрос macro_rules!: список правил «образец — подстановка»."
[g-layout]: ../appendix-f.md#layout "Раскладка (layout): Размер, выравнивание и смещения полей типа в памяти."
[g-lint]: ../appendix-f.md#lint "Линт (lint): Правило, которое срабатывает на подозрительном, но допустимом коде: unused_comparisons в rustc, manual_memcpy в clippy."
[g-module]: ../appendix-f.md#module "Модуль (module): Пространство имён внутри крейта со своими правилами видимости: объявляется словом mod, с телом в скобках или в отдельном файле."
[g-prelude]: ../appendix-f.md#prelude "Прелюдия (prelude): Набор имён, которые видны в каждом модуле без use: Option, Vec, drop, трейты Clone, Iterator и другие."
