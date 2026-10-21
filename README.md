# Rust для системного программиста

Учебник Rust для студентов, которые знают C и знакомы с устройством ОС.
Язык разбирается на уровне механизмов: как устроены типы в памяти, во что
компилируется код, где проходит граница `unsafe`. Каждое утверждение
книги проверяется кодом из этого репозитория: тестами, Miri, листингами
ассемблера для x86_64 и ARM Cortex-M и прошивками в QEMU.

В каждой главе есть задачи от простых к сложным, разборы всех решений
открыто лежат в приложении E. Через всю книгу проходит сквозной проект
«станция телеметрии»: прошивка Cortex-M собирает измерения, хост на Linux
их принимает.

## Состояние

- Программа курса: глава 0, 31 глава в шести частях, приложения A–E.
- Готова эталонная глава 6 «Составные типы и раскладка памяти»:
  11 разделов, задачи A–D, первая задача проекта П6 и разборы в
  приложении E. Остальные главы пока содержат план из программы курса.

## Чтение

```console
$ mdbook serve --open                 # книга на http://localhost:3000
$ mdbook build                        # статический сайт в каталоге book/
```

## Устройство репозитория

```text
src/                      текст книги (mdBook)
  ch06/                   эталонная глава, по файлу на раздел
  appendix-e/             разборы задач всех глав
code/chNN/                примеры главы; листинги попадают в текст через {{#include}}
code/ch06-regs/           драйвер UART PL011 из раздела 6.9
exercises/chNN/           заготовки задач
exercises/project-frame/  заготовка задачи П6
solutions/chNN/           эталонные решения
tests-shared/             тесты, общие для заготовок и решений
project/                  сквозной проект «станция телеметрии»
firmware/                 прошивки для QEMU (отдельный workspace)
theme/                    стили врезок: справка, сравнение с C, под капотом, трюк, ловушка
tools/                    вспомогательные проверки для CI
```

## Окружение

Установка на Arch Linux и Debian/Ubuntu описана в
[приложении D](src/appendix-d.md). Коротко:

- Rust 1.97.0 и три цели Cortex-M закреплены в `rust-toolchain.toml`,
  rustup поставит их сам;
- nightly с Miri для проверки `unsafe` и флагов `-Z`;
- `qemu-system-arm` 8.x, машины `lm3s6965evb` (Cortex-M3) и `microbit`
  (Cortex-M0);
- mdBook 0.5.4 и `cargo-show-asm`.

## Задачи

```console
$ cargo test -p ch06-exercises            # все задачи главы 6
$ cargo test -p ch06-exercises task_b     # одна задача
$ cargo test -p project-frame             # задача П6
```

Пока функции не реализованы, тесты падают на `todo!()`. Тесты общие для
заготовок и эталонных решений, поэтому зелёный прогон значит то же, что и
у автора.

## Проверки

Те же команды выполняет CI (`.github/workflows/ci.yml`):

```console
$ cargo fmt --all --check && (cd firmware && cargo fmt --all --check)
$ cargo clippy --workspace --all-targets -- -D warnings
$ cargo test                              # примеры глав, решения, проект
$ cargo +nightly miri test -p ch06 -p ch06-regs -p ch06-solutions -p telemetry-frame
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch06-solutions
$ cargo build -p ch06 --no-default-features --target thumbv6m-none-eabi
$ bash firmware/run-demos.sh              # 8 опытов главы 6 в QEMU
$ python3 tools/check-includes.py         # все {{#include}} указывают на существующие якоря
$ mdbook build
```

Задание CI со случайной раскладкой полей прогоняет решения под тремя
зёрнами: эталонный код не должен зависеть от раскладки `repr(Rust)`.
Тесты группы `observed` в `code/ch06` фиксируют поведение текущего rustc и
под случайной раскладкой падают намеренно (раздел 6.2), поэтому там они
пропускаются.

## Лицензия

Код примеров, заготовок и решений объявлен в `Cargo.toml` под
MIT OR Apache-2.0. Лицензия текста книги пока не выбрана.
