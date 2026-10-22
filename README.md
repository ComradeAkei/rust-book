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
  приложении E. Остальные главы, кроме 0–5, пока содержат план из
  программы курса.
- Готовы главы 0–2: окружение и инструменты (rustup, cargo, чтение
  ассемблера, одна программа на x86_64 и Cortex-M, Miri и clippy, QEMU и
  gdb), скалярные типы и их представление в памяти, выражения и
  сопоставление с образцом вместе с тем, во что компилируется `match`.
  В каждой главе задачи A–D (в главе 1 — A, B1, B2, C, D), эталонные
  решения и разборы в приложении E.
- Готовы главы 3–5: владение и перемещение (drop и его флаги, перемещение
  в машинном коде, стек прошивки и стражи прерываний на Cortex-M),
  заимствование и ссылки (правило читателей и писателя, NLL, раздельные
  заимствования, `noalias` и границы проверки, Polonius), времена жизни
  (элизия, разбор без копирования, `'static` во flash, вариантность,
  `for<'a>`, GAT, захват в `impl Trait`). Задачи A–D, решения и разборы
  в приложении E, прошивки в `firmware/ch03-*` и `firmware/ch05-addr`.
- Готовы приложения A–D: словарь соответствий C → Rust, edition 2024 для
  системного кода, чек-лист ревью `unsafe` с разбором примера, установка
  окружения и отладка в QEMU. Их код лежит в `code/appendix-*`.

## Чтение

```console
$ mdbook serve --open                 # книга на http://localhost:3000
$ mdbook build                        # статический сайт в каталоге book/
```

## Устройство репозитория

```text
src/                      текст книги (mdBook)
  ch00/ ch01/ ch02/       главы 0–2, по файлу на раздел
  ch03/ ch04/ ch05/       главы 3–5, по файлу на раздел
  ch06/                   эталонная глава, по файлу на раздел
  appendix-e/             разборы задач всех глав
code/chNN/                примеры главы; листинги попадают в текст через {{#include}}
code/ch01/c/ … ch05/c/    те же примеры на C и run.sh, который сверяет их вывод с текстом
code/ch0[45]/rejected/    программы, которые проверка отвергает; их собирают тесты
code/ch0[45]/fragments/   фрагменты к задачам A глав 4 и 5
code/ch06-regs/           драйвер UART PL011 из раздела 6.9
code/appendix-a/          словарь C → Rust; в c/ те же примеры на C и run.sh для UBSan
code/appendix-b/          edition 2024 для системного кода
code/appendix-b-2021/     те же исходники в edition 2021 для сравнения
code/appendix-c/          вектор фиксированной ёмкости до и после ревью unsafe
exercises/chNN/           заготовки задач
exercises/ch02-2021/      задача 2.D в edition 2021: тот же файл, что в exercises/ch02
exercises/project-frame/  заготовка задачи П6
solutions/chNN/           эталонные решения
solutions/ch02-2021/      решение задачи 2.D в edition 2021
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
  (Cortex-M0), для главы 0 ещё `mps2-an386` (Cortex-M4F), для C-примеров
  главы 1 `mps2-an385` (Cortex-M3);
- `gdb-multiarch` для задачи 0.D, `llvm` (`llvm-objdump`, `llvm-nm`) для
  задачи 2.B, GCC и clang для C-примеров; `arm-none-eabi-gcc` с newlib
  нужен только для проверки C-примеров глав 1 и 4 на Cortex-M;
- mdBook 0.5.4 и `cargo-show-asm`.

## Задачи

```console
$ cargo test -p ch00-exercises            # задачи A–C главы 0; D проверяет gdb
$ cargo test -p ch01-exercises            # все задачи главы 1
$ cargo test -p ch02-exercises            # все задачи главы 2
$ cargo test -p ch02-exercises-2021       # задача 2.D в edition 2021
$ cargo test -p ch03-exercises            # все задачи главы 3
$ cargo test -p ch04-exercises            # все задачи главы 4
$ cargo test -p ch05-exercises            # все задачи главы 5
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
$ cargo test -p appendix-a --release      # часть поведения приложения A зависит от профиля
$ cargo +nightly miri test -p ch00 -p ch01 -p ch02 -p ch00-solutions -p ch01-solutions \
      -p ch02-solutions -p ch02-solutions-2021
$ cargo +nightly miri test -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions \
      -p ch05-solutions                   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch06 -p ch06-regs -p ch06-solutions -p telemetry-frame
$ cargo +nightly miri test -p appendix-a -p appendix-b -p appendix-b-2021 -p appendix-c
$ MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test -p appendix-c
$ cargo +nightly test --doc -p ch00 -p ch01 -p ch02 -p ch06 -p appendix-a -p appendix-b \
      -p appendix-b-2021 -p appendix-c
$ cargo +nightly test --doc -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions \
      -p ch05-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch06-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch00-solutions -p ch01-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch02-solutions -- --skip task_a
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch03 -p ch04 -p ch05 \
      -p ch05-solutions -- --skip polonius
$ cargo build -p ch06 --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch00 -p ch01 -p ch02 -p ch00-solutions -p ch01-solutions --target thumbv6m-none-eabi
$ cargo build -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions -p ch05-solutions \
      --target thumbv6m-none-eabi
$ bash firmware/run-demos.sh              # опыты глав 0, 3, 5 и 6 в QEMU: 6, 5, 2 и 8
$ cargo test -p ch02-solutions -- --ignored own_firmware   # задача 2.B на всех прошивках
$ bash code/appendix-a/c/run.sh           # C-примеры приложения A под UBSan
$ bash code/ch01/c/run.sh                 # C-примеры главы 1 на x86_64 и в QEMU
$ bash code/ch02/c/run.sh                 # switch из введения к главе 2
$ bash code/ch03/c/run.sh                 # queue.c из введения к главе 3: ASan и анализаторы
$ bash code/ch04/c/run.sh                 # samples.c, count.c и overlap.c из главы 4
$ bash code/ch05/c/run.sh                 # config.c из введения к главе 5
$ bash solutions/ch01/c/run.sh            # задача 1.A на C
$ python3 tools/check-includes.py         # все {{#include}} указывают на существующие якоря
$ mdbook build
```

Задание CI со случайной раскладкой полей прогоняет решения под тремя
зёрнами: эталонный код не должен зависеть от раскладки `repr(Rust)`.
Тесты группы `observed` в `code/ch06` фиксируют поведение текущего rustc и
под случайной раскладкой падают намеренно (раздел 6.2), поэтому там они
пропускаются. В главах 3–5 там же пропускаются тесты, которые запускают
закреплённый rustc или сверяют ассемблер с ответом (Polonius, задачи 3.C,
4.A и 4.B): к раскладке они не относятся, а в этом задании установлен
только nightly. Их проверяет основное задание.

## Лицензия

Код примеров, заготовок и решений объявлен в `Cargo.toml` под
MIT OR Apache-2.0. Лицензия текста книги пока не выбрана.
