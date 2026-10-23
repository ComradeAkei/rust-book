# 24.4. Атрибуты: помощники и `#[interrupt]`

## Атрибуты-помощники

Derive не может получить аргументы в `#[derive(...)]`, поэтому настройки
передают **атрибутами-помощниками**. `#[proc_macro_derive(Describe,
attributes(describe))]` объявляет имя `describe`: на структуре с этим
derive и на её полях компилятор принимает `#[describe(...)]` и ничего с
ним не делает — атрибут инертный, его читает только derive. Без derive тот
же атрибут — ошибка «cannot find attribute».

Содержимое разбирает `parse_nested_meta` из syn — разборщик синтаксиса
`ключ`, `ключ = значение` и `ключ(...)` через запятую:

```rust,ignore
{{#include ../../code/ch24-macros/src/describe.rs:opts}}
```

У структуры два ключа: `size = N` (проверка размера из раздела
[24.3](03-spans.md)) и `crate = путь` (ниже), у поля — `skip`. Значения
разбираются как узлы syn — `LitInt`, `Path`, — и неверный тип значения
syn отвергает сам, со своим текстом и спаном. Ошибка в одном атрибуте не
останавливает разбор остальных: `struct_opts` копит ошибки через
`combine`, и пользователь видит все три из снимка `describe_attrs` сразу.

`skip` выключает поле из описания — например, поле без `Debug`:

```rust,ignore
struct Opaque;
#[derive(ch24::Describe)]
struct Holder { id: u8, #[describe(skip)] _raw: Opaque }
assert_eq!(<Holder as ch24::Describe>::FIELDS.len(), 1);
```

### Путь к своему крейту

У пользователя, который подключил `ch24` под другим именем, пути `::ch24`
нет:

```toml
{{#include ../../code/ch24/renamed/Cargo.toml}}
```

```rust,ignore
{{#include ../../code/ch24/renamed/src/lib.rs:broken}}
```

```text
error[E0433]: cannot find `ch24` in the crate root
  --> src/lib.rs:18:10
   |
18 | #[derive(station::Describe)]
   |          ^^^^^^^^^^^^^^^^^ could not find `ch24` in the list of imported crates
   |
   = note: this error originates in the derive macro `station::Describe` (in Nightly builds, run with -Z macro-backtrace for more info)
```

Макрос не может узнать, как зависимость названа у вызывающего: он видит
только токены. Поэтому имя называет пользователь:

```rust,ignore
{{#include ../../code/ch24/renamed/src/lib.rs:renamed}}
```

Тест `rustc_crate_path_attribute` собирает крейт `code/ch24/renamed`
дважды: с атрибутом — успешно, со структурой `Broken` — с ошибкой E0433.

## Атрибут на функции: `#[interrupt]`

В прошивке на cortex-m-rt обработчик прерывания выглядит так:

```rust,ignore
use lm3s6965::interrupt;

#[interrupt]
fn UART0() {
    static mut COUNT: u32 = 0;
    *COUNT += 1;
}
```

Атрибут проверяет сигнатуру, сверяет имя со списком прерываний
устройства, превращает `static mut` из начала тела в `&mut` без `unsafe`
и экспортирует функцию под именем, которое ждёт таблица векторов.
`ch24-macros` повторяет всё это для учебного устройства. Список
прерываний — перечисление в крейте устройства (в настоящей прошивке его
генерирует svd2rust, раздел [24.7](07-tasks.md)):

```rust,ignore
{{#include ../../code/ch24/src/device.rs:device}}
```

Одно имя `interrupt` экспортировано дважды — перечисление и атрибут, —
и конфликта нет: у типов и макросов разные пространства имён. Строка `use
ch24::device::interrupt;` приносит пользователю оба.

### Сигнатура

```rust,ignore
{{#include ../../code/ch24-macros/src/interrupt.rs:check}}
```

Обработчик вызывает процессор, поэтому видимость, `const`, `async`,
своё ABI, параметры типа и аргументы бессмысленны, а результат может
быть только `()` или `!`. Каждое нарушение — ошибка со спаном той части
сигнатуры, которую надо исправить, и все вместе:

```text
{{#include ../../code/ch24/tests/ui/interrupt_signature.stderr}}
```

Это и есть «макрос проверяет правила за человека»: в C обработчик с
аргументом скомпилируется и будет читать из `r0`–`r3` то, что оставил в
них прерванный код.

### Имя

Имя прерывания проверяет не макрос, а компилятор. Макрос не знает списка
прерываний — тот в другом крейте, — но может вставить в тело обращение к
нему: `let _ = interrupt::UART0;`. Путь `interrupt::` разрешается там, где
стоит атрибут, то есть у пользователя, а токен имени — тот, что написал
пользователь, со своим спаном. Опечатка:

```text
{{#include ../../code/ch24/tests/ui/interrupt_unknown.stderr}}
```

Ошибка указывает на имя функции, а подсказку «there is a variant with a
similar name» даёт компилятор — бесплатно, потому что проверку выполнил
он. Так же поступает cortex-m-rt. Обратная сторона та же, что у него:
атрибут, названный полным путём без `use`, — `#[ch24::device::interrupt]`
— раскрывается в `interrupt::UART1`, а `interrupt` в модуле пользователя
не объявлен, и сборка падает с E0433 (doc-тест в `device.rs`).

### Раскрытие

```rust,ignore
{{#include ../../code/ch24-macros/src/interrupt.rs:expand}}
```

Функция пользователя переименована в `__ch24_UART0`, и её не вызвать по
старому имени. `static mut` из начала тела стали её параметрами `&mut T`,
а значения для них создаёт **переходник** — функция без аргументов с
ABI C, экспортированная под символом `UART0`. Для обработчика:

```rust,ignore
{{#include ../../code/ch24/src/device.rs:handler}}
```

раскрытие (снимок `code/ch24/expanded/interrupt_uart0.rs`, его сверяет
тест `rustc_expansions_match_snapshots`):

```rust,ignore
{{#include ../../code/ch24/expanded/interrupt_uart0.rs}}
```

Каждый `static mut` объявлен *внутри блока* в аргументе вызова: он
существует один на всю программу, но имя видно только этому блоку, и
другого пути к нему нет. `&mut *&raw mut COUNT` берёт ссылку через сырой
указатель: в edition 2024 прямое `&mut COUNT` на `static mut` — ошибка
(линт `static_mut_refs`, разделы [16.1](../ch16/01-abilities.md) и
[16.7](../ch16/07-edition.md)).
Единственность ссылки обеспечивает железо: процессор не входит в
обработчик повторно, пока тот не вернулся, а переходник — `unsafe
extern "C"`, и вызвать его из безопасного кода нельзя. Комментарий
`SAFETY` из шаблона в раскрытии пропал: комментарии не токены.

Тест вызывает переходник, как процессор, — по символу:

```rust,ignore
{{#include ../../code/ch24/src/device.rs:handler_test}}
```

<div class="box trap">

**Ловушка — редакция сгенерированных токенов.** В edition 2024
`#[export_name]`, `#[no_mangle]` и `#[link_section]` пишут как
`#[unsafe(...)]` ([16.7](../ch16/07-edition.md)): экспортированное имя
может совпасть с чужим символом и подменить его, и за это отвечает
программист. Какие правила применить к токену, компилятор
решает по редакции, записанной в его спане, а у токенов, созданных
макросом, это редакция *крейта макроса*. `ch24-macros` — крейт 2024, и
без `unsafe(...)` его выход не компилируется: «unsafe attribute used
without unsafe». cortex-m-rt-macros 0.7.7 — крейт 2021 и порождает
`#[export_name = "main"]` без `unsafe`, и это проходит в прошивке 2024
(задача A). Обновив редакцию крейта макросов, проверьте его выход заново.

</div>

<div class="box c">

**Как в C.** В стартовом файле CMSIS каждое прерывание — слабый символ:
`.weak UART0_IRQHandler` и `.thumb_set UART0_IRQHandler, Default_Handler`.
Свой обработчик с тем же именем его перекрывает, а с опечаткой —
`UART0_IRQHandlr` — просто становится ещё одной функцией, которую никто
не вызывает. В Rust устроено похоже: таблицу векторов собирает крейт
устройства, и его скрипт компоновки `device.x` задаёт значения по
умолчанию — `PROVIDE(UART0 = DefaultHandler);` (так пишет svd2rust).
Но имя проверено до компоновки: переходник экспортирован, только если
`interrupt::UART0` существует.

</div>

<div class="box check">

**Проверка.** Тесты `handler_keeps_state_between_calls` (три вызова через
символ — счётчик 3) и `diverging_handler_compiles` в
`code/ch24/src/device.rs`, там же doc-тесты `compile_fail,E0599`,
`compile_fail` (сигнатура) и `compile_fail,E0433` (атрибут без `use`);
снимки `interrupt_signature`, `interrupt_unknown` и `describe_attrs` —
тест `rustc_ui`; `rustc_crate_path_attribute` — переименованный крейт.

</div>
