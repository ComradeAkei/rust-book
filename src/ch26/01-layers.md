# 26.1. core, alloc и std

## Три слоя

| Слой | Что даёт | Что требует от среды |
|---|---|---|
| `core` | типы и трейты языка, `Option`, `Result`, срезы, `str`, итераторы, `core::fmt`, атомики, `Cell` | обработчик паники и функции памяти: `memcpy`, `memmove`, `memset`, `memcmp`, `bcmp`, `strlen` |
| `alloc` | `Box`, `Vec`, `String`, `Rc`, `BTreeMap`, `format!` | глобальный распределитель памяти |
| `std` | всё из `core` и `alloc`, плюс потоки, файлы, сеть, часы, окружение, раскрутка стека | операционную систему |

`std` не отдельная библиотека с другими `Vec` и `Option`: она
переэкспортирует `core` и `alloc` и добавляет то, что требует ОС.
`std::vec::Vec` и `alloc::vec::Vec` — один тип. Поэтому код, написанный
на `core`, работает и в программе со `std` — и обратного направления
достаточно, чтобы библиотеку можно было использовать везде.

Функции памяти `core` требует потому, что компилятор вставляет их
вызовы сам (раздел [26.5](05-memcpy.md)). На хосте их даёт libc, на
голом железе — `compiler_builtins`. Обработчик паники объявляет
программа: `core` знает, *как* сообщить о панике, но не знает, *куда*.

## Ошибки слоёв

В крейте с `#![no_std]` в прелюдии только `core`:

```rust,ignore
{{#include ../../code/ch26/fragments/vec.rs:vec}}
```

```text
error[E0425]: cannot find type `Vec` in this scope
 --> fragments/vec.rs:6:12
  |
6 |     let v: Vec<u16> = Vec::new();
  |            ^^^ not found in this scope

error[E0433]: cannot find type `Vec` in this scope
 --> fragments/vec.rs:6:23
  |
6 |     let v: Vec<u16> = Vec::new();
  |                       ^^^ use of undeclared type `Vec`
```

Это сборка под x86-64: ОС ни при чём, `Vec` просто нет в области
видимости. Подсказки «подключите `alloc`» rustc 1.97 не даёт. `Vec`
живёт в `alloc`, и крейт подключается явно:

```rust,ignore
{{#include ../../code/ch26/fragments/vec_alloc.rs:vec_alloc}}
```

Библиотека собирается и под Cortex-M0. А программа с ней — уже нет,
пока не объявлен распределитель:

```rust,ignore
{{#include ../../code/ch26/fragments/no_allocator.rs:no_allocator}}
```

```text
error: no global memory allocator found but one is required; link to std or add `#[global_allocator]` to a static item that implements the GlobalAlloc trait
```

Ошибка — при сборке программы, а не библиотеки: распределитель один на
программу, и выбирает его тот, кто собирает исполняемый файл. Кучу для
Cortex-M с крейтом `embedded-alloc` разбирал раздел
[15.8](../ch15/08-nostd.md).

`std` для голого железа нет вовсе — ни в каком виде:

```rust,ignore
#![no_std]

extern crate std;
```

```text
error[E0463]: can't find crate for `std`
 --> fragments/std_on_m0.rs:5:1
  |
5 | extern crate std;
  | ^^^^^^^^^^^^^^^^^ can't find crate
  |
  = note: the `thumbv6m-none-eabi` target may not support the standard library
```

<div class="box trap">

**Ловушка.** Эту же ошибку даёт зависимость, которая по умолчанию
включает свою возможность `std`: в её `lib.rs` стоит
`#[cfg(feature = "std")] extern crate std;`, и сборка под Cortex-M
падает в чужом крейте. Лечится в своём `Cargo.toml`: `default-features =
false`, как у прошивки `ch26-bare`, которая зависит от крейта этой главы.
Возможности объединяются по всему графу зависимостей (раздел
[8.4](../ch08/04-features.md)), и одной зависимости с `std` по умолчанию
достаточно.

</div>

## Слой — не только ОС, но и процессор

`core` одинакова для всех целей по набору модулей, но не по содержимому
— часть элементов зависит от того, что умеет процессор. У ARMv6-M
(Cortex-M0) нет инструкций `ldrex` и `strex`, а значит, нет атомарного
чтения-изменения-записи:

```rust,ignore
{{#include ../../code/ch26/fragments/atomics_m0.rs:atomics_m0}}
```

Под thumbv7m-none-eabi это собирается, под thumbv6m-none-eabi — нет:

```text
error[E0432]: unresolved import `alloc::sync`
   --> fragments/atomics_m0.rs:8:12
    |
  8 | use alloc::sync::Arc;
    |            ^^^^ could not find `sync` in `alloc`
    |
note: found an item that was configured out
   --> …/library/alloc/src/lib.rs:239:9
    |
238 | #[cfg(all(not(no_rc), not(no_sync), target_has_atomic = "ptr"))]
    |                                     ------------------------- the item is gated behind the `ptr` feature
239 | pub mod sync;
    |         ^^^^

error[E0599]: no method named `fetch_add` found for reference `&Atomic<u32>` in the current scope
  --> fragments/atomics_m0.rs:16:7
   |
16 |     c.fetch_add(1, Ordering::Relaxed)
   |       ^^^^^^^^^ method not found in `&Atomic<u32>`
```

Модуля `alloc::sync` с `Arc` на M0 нет: счётчик ссылок `Arc` меняется
атомарно, а `target_has_atomic = "ptr"` означает именно атомарные
операции над словом, не только `load` и `store`. `AtomicU32` на M0 есть,
но только с `load`, `store` и без `fetch_add` и `compare_exchange` —
отсюда `Atomic<u32>` в сообщении: в 1.97 `AtomicU32` — псевдоним
обобщённого `Atomic<u32>`. Атомики на Cortex-M разбирала глава
[21](../ch21/index.md); прошивки этой книги на M0 считают через `load` и
`store` при выключенных прерываниях или без прерываний вовсе.

## Один отчёт на трёх слоях

Библиотека главы — `no_std` всегда, а `alloc` и `std` подключают
возможности с теми же именами:

```rust,ignore
{{#include ../../code/ch26/src/lib.rs:crate}}
```

```toml
{{#include ../../code/ch26/Cargo.toml:features}}
```

`std = ["alloc"]`: кто включил `std`, получает и `alloc`. По умолчанию
включена `std` — на хосте крейт ведёт себя как обычный, а прошивка
выключает её. Отчёт о показании датчика — на каждом слое свой:

```rust,ignore
{{#include ../../code/ch26/src/layers.rs:cursor}}
```

На слое `core` форматирование есть — `write!` работает с любым
`fmt::Write`, — а памяти для результата нет. Её даёт вызывающий: срез на
стеке. `Cursor` пишет в него и на нехватке места возвращает
`fmt::Error`, а не расширяет буфер. Это `snprintf` в `char[32]`, только
переполнение — ошибка, а не молча обрезанная строка.

```rust,ignore
{{#include ../../code/ch26/src/layers.rs:alloc}}
```

На слое `alloc` память берётся из кучи, и функция возвращает `String`. На
слое `std` можно печатать. Тест `three_layers_agree` проверяет, что три
функции дают один и тот же текст «канал 3: -12.05», а
`small_buffer_is_an_error` — что буфер на 8 байт для «канал 1: 0.07»
(13 символов, но 18 байт: кириллица по два) — ошибка, а на 18 — нет.

```console
$ cargo build -p ch26 --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch26 --no-default-features --features alloc --target thumbv6m-none-eabi
```

Обе сборки проходят, и CI повторяет их под три цели Cortex-M.

<div class="box c">

**Как в C.** В C тот же раздел проводят файлами: `report.c` с
`snprintf` компилируется везде, `report_print.c` с `printf` — только там,
где есть `stdout`, а `#ifdef HAVE_MALLOC` выбирает, кто выделяет буфер.
Но и «чистый» `snprintf` не обязательно чист: прошивка с одним
`snprintf(buf, sizeof buf, "%d", x)` из newlib (arm-none-eabi-gcc 13.2,
и полная newlib, и `--specs=nano.specs`) содержит `_malloc_r` и `_sbrk`
— `code/ch26/c/run.sh` проверяет это по `nm`. В Rust `core::fmt` не
выделяет память никогда, а `format!` без `alloc` просто не найдётся.

</div>

<div class="box check">

**Проверь себя.** `Cursor::into_str` вызывает
`core::str::from_utf8(…).unwrap()`. Почему здесь `unwrap` никогда не
паникует? Почему прошивка `ch26-bare` для своих цифр (раздел
[26.3](03-cortex-m.md)) всё-таки не пользуется `from_utf8(…).unwrap()`?

</div>
