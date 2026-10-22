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
  приложении E. Остальные главы, кроме 0–11, пока содержат план из
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
- Готова глава 7 «Обработка ошибок и паника»: `Result` и `?` в машинном
  коде, `From` и `Box<dyn Error>`, цепочки `source`, размер ошибки,
  паника и раскрутка по таблицам `.eh_frame`, граница FFI и
  `extern "C-unwind"`, `#[panic_handler]` на Cortex-M. Задачи A–D,
  второй шаг проекта П7 (ошибки протокола), прошивки
  `firmware/ch07-panic` и `firmware/ch07-frame`.
- Готова глава 8 «Модули, крейты и сборка»: видимость и что она делает
  с символами, крейты и `Cargo.lock`, `cfg` и `target_has_atomic` на
  Cortex-M0, аддитивность возможностей, `build.rs` на хосте и для цели,
  `rlib`, `staticlib` и `cdylib` изнутри, манглинг v0 и legacy,
  столкновения имён без манглинга, секции и `.uninit`. Задачи A–D
  (видимость, таблица синуса для M0, staticlib для программы на C с
  разбором имён, неаддитивные возможности), прошивка
  `firmware/ch08-sections`.
- Готова глава 9 «Трейты»: реализации и статическая диспетчеризация в
  машинном коде против таблицы указателей на функции в C,
  ассоциированные типы и константы (регистры SysTick как типы с
  проверкой выравнивания при мономорфизации), супертрейты, правило
  сироты и когерентность, маркерные трейты и `unsafe impl Send`,
  операторы Q15 с DSP-инструкциями Cortex-M4, `From`, `AsRef`, `Borrow`,
  `derive` изнутри, запечатанные трейты и трейты-расширения. Задачи A–D
  (шесть пар реализаций, регистры PL011 как типы, запечатанные выводы
  GPIO и битовые поля, драйвер датчика для I2C и SPI поверх
  `embedded-hal` 1.0), прошивка `firmware/ch09-systick`.
- Готова глава 10 «Обобщения и мономорфизация»: `qsort` с компаратором
  через указатель и сортировка, размноженная макросом, против обобщений
  с ограничениями, `where` и условные методы, `impl Trait` в аргументе,
  результате и методах трейтов, параметры-константы (маска вместо
  деления, проверки при мономорфизации), копии в бинарнике по `nm` и
  `cargo bloat`, слияние копий и внутренняя необобщённая функция,
  встраивание замыкания против указателя на функцию, typestate для UART
  PL011 без единой лишней инструкции. Задачи A–D (сколько копий в шести
  программах, `qsort` против `sort_unstable_by` на 10 миллионах чисел,
  матрицы с размерностями в типе, typestate-драйвер SPI-flash W25Q
  поверх `embedded-hal` 1.0), прошивка `firmware/ch10-typestate`.
- Готова глава 11 «Динамическая диспетчеризация»: `struct
  file_operations` и цена вызова через таблицу на C, толстые указатели и
  их время жизни, содержимое таблицы методов в ассемблере x86-64 и
  Cortex-M, `drop` и освобождение `Box<dyn>`, таблицы во flash,
  совместимость с `dyn` и `where Self: Sized`, `?Sized` и пересылающие
  реализации, `Rc<dyn>`, `&mut dyn FnMut` без кучи, `Any` и приведение к
  супертрейту, предсказатель косвенных переходов, девиртуализация и
  retpoline, перечисление вместо `dyn`. Задачи A–D (размеры указателей,
  таблица методов вручную, `dyn` против перечисления и обобщений на 2, 8
  и 64 типах, размер из таблицы под Miri), прошивка `firmware/ch11-dyn`.
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
  ch07/ ch08/ ch09/       главы 7–9, по файлу на раздел
  ch10/ ch11/             главы 10 и 11, по файлу на раздел
  appendix-e/             разборы задач всех глав
code/chNN/                примеры главы; листинги попадают в текст через {{#include}}
code/ch01/c/ … ch11/c/    те же примеры на C и run.sh, который сверяет их вывод с текстом
code/ch0[45]/rejected/    программы, которые проверка отвергает; их собирают тесты
code/ch0[45789]/fragments/  фрагменты к задачам A глав 4, 5, 7, 8 и 9
code/ch10/fragments/      шесть программ задачи 10.A: сколько копий обобщённой функции
code/ch10/tests/          замеры кода для Cortex-M3: typestate и внутренняя функция
code/ch11/tests/          ассемблер вызовов через таблицу; на nightly — DynMetadata и retpoline
code/ch07/c-size/         программа задачи 7.C: размер при panic = "unwind" и "abort"
code/ch07/hir/            во что раскрывается `?`: снимок HIR для раздела 7.1
code/ch08/typo/ unify/ ffi/  отдельные рабочие пространства для опытов главы 8 с cargo
code/ch08/dup/            столкновение имён без манглинга: исходники для rustc
code/ch09/expanded/       что пишет derive: исходник и снимок -Zunpretty=expanded
code/ch06-regs/           драйвер UART PL011 из раздела 6.9
code/appendix-a/          словарь C → Rust; в c/ те же примеры на C и run.sh для UBSan
code/appendix-b/          edition 2024 для системного кода
code/appendix-b-2021/     те же исходники в edition 2021 для сравнения
code/appendix-c/          вектор фиксированной ёмкости до и после ревью unsafe
exercises/chNN/           заготовки задач
exercises/ch02-2021/      задача 2.D в edition 2021: тот же файл, что в exercises/ch02
exercises/ch08/c/         задача 8.C: программа на C, Makefile и крейт station
exercises/ch08/d-station/ задача 8.D: рабочее пространство с неаддитивными возможностями
exercises/project-frame/  заготовка задач П6 и П7
solutions/chNN/           эталонные решения
solutions/ch02-2021/      решение задачи 2.D в edition 2021
solutions/ch08/c/ d-station/  решения задач 8.C и 8.D
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
$ cargo test -p ch07-exercises            # все задачи главы 7
$ cargo test -p ch08-exercises            # все задачи главы 8
$ cargo test -p ch09-exercises            # все задачи главы 9
$ cargo test -p ch10-exercises            # все задачи главы 10
$ cargo run --release -p ch10-exercises --example b_bench   # время для задачи 10.B
$ cargo test -p ch11-exercises            # все задачи главы 11
$ cargo run --release -p ch11-exercises --example c_bench   # время для задачи 11.C
$ cargo test -p project-frame             # задачи П6 и П7
$ cargo test -p project-frame p7          # только П7
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
$ cargo +nightly miri test -p ch07 -p ch07-solutions   # и ещё раз с MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test -p ch08 -p ch08-solutions
$ cargo +nightly miri test -p ch09 -p ch09-solutions
$ cargo +nightly miri test -p ch10 -p ch10-solutions
$ cargo +nightly miri test -p ch11 -p ch11-solutions
$ cargo +nightly miri test -p appendix-a -p appendix-b -p appendix-b-2021 -p appendix-c
$ MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test -p appendix-c
$ cargo +nightly test --doc -p ch00 -p ch01 -p ch02 -p ch06 -p ch07 -p ch08 -p ch09 \
      -p ch10 -p ch11 -p appendix-a -p appendix-b -p appendix-b-2021 -p appendix-c
$ cargo +nightly test -p ch11 --features nightly --test nightly   # DynMetadata и -Zretpoline
$ cargo +nightly test --doc -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions \
      -p ch05-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch06-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch00-solutions -p ch01-solutions
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch02-solutions -- --skip task_a
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch03 -p ch04 -p ch05 \
      -p ch05-solutions -- --skip polonius
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch07 -p ch07-solutions \
      -- --skip task_a --skip task_c --skip observed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch08 -p ch08-solutions \
      -- --skip cargo_ --skip task_a --skip task_b_flash --skip task_c_from_c --skip task_d
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch09 -p ch09-solutions \
      -- --skip rustc_ --skip task_a --skip task_b_access --skip task_b_thumb --skip task_c_sealed
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch10 -p ch10-solutions \
      -- --skip rustc_ --skip task_a --skip task_b_inlined --skip task_c_types --skip task_c_sizes \
      --skip task_d_types
$ RUSTFLAGS="-Zrandomize-layout -Zlayout-seed=1" cargo +nightly test -p ch11 -p ch11-solutions \
      -- --skip rustc_ --skip task_a_thumb --skip task_b_same_code --skip task_c_calls
$ cargo build -p ch06 -p ch07 --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch08 -p ch08-solutions --no-default-features --target thumbv6m-none-eabi
$ cargo build -p ch09 --no-default-features --features alloc --target thumbv6m-none-eabi
$ cargo build -p ch09-solutions -p ch09-exercises --target thumbv6m-none-eabi
$ cargo build -p ch10 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc
$ cargo build -p ch10-solutions -p ch10-exercises --target thumbv6m-none-eabi
$ cargo build -p ch11 --no-default-features --target thumbv6m-none-eabi   # и с --features alloc
$ cargo build -p ch11-solutions -p ch11-exercises --target thumbv6m-none-eabi
$ cargo build -p ch00 -p ch01 -p ch02 -p ch00-solutions -p ch01-solutions --target thumbv6m-none-eabi
$ cargo build -p ch03 -p ch04 -p ch05 -p ch03-solutions -p ch04-solutions -p ch05-solutions \
      --target thumbv6m-none-eabi
$ bash firmware/run-demos.sh              # опыты глав 0, 3, 5–11 в QEMU: 6, 5, 2, 8, 11, 7, 3, 2 и 5
$ cargo test -p ch02-solutions -- --ignored own_firmware   # задача 2.B на всех прошивках
$ bash code/appendix-a/c/run.sh           # C-примеры приложения A под UBSan
$ bash code/ch01/c/run.sh                 # C-примеры главы 1 на x86_64 и в QEMU
$ bash code/ch02/c/run.sh                 # switch из введения к главе 2
$ bash code/ch03/c/run.sh                 # queue.c из введения к главе 3: ASan и анализаторы
$ bash code/ch04/c/run.sh                 # samples.c, count.c и overlap.c из главы 4
$ bash code/ch05/c/run.sh                 # config.c из введения к главе 5
$ bash code/ch07/c/run.sh                 # errno.c, unchecked.c и longjmp.c из главы 7
$ bash code/ch08/c/run.sh                 # ifdef.c и столкновение глобальных имён из главы 8
$ bash code/ch09/c/run.sh                 # ops.c: таблица указателей на функции из главы 9
$ bash code/ch10/c/run.sh                 # sort.c: qsort против макроса из главы 10
$ bash code/ch11/c/run.sh                 # fops.c: таблица, предсказатель и retpoline из главы 11
$ cargo fmt --all --check --manifest-path code/ch08/ffi/Cargo.toml   # и так каждое вложенное
      # рабочее пространство: code/ch07/c-size, code/ch08/{typo,unify,ffi},
      # {exercises,solutions}/ch08/c/station, {exercises,solutions}/ch08/d-station
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
4.A и 4.B), в главе 7 — задачи 7.A и 7.C, в главе 8 — опыты с cargo и
задачи, которые запускают rustc, cargo или `make`, в главе 9 — снимок
`derive` и пробы задач 9.A, 9.B и 9.C, в главах 10 и 11 — замеры кода и задачи,
которые собирают пробы или крейт закреплённым тулчейном: к раскладке они
не относятся, а в этом задании установлен только nightly. Их проверяет
основное задание.

## Лицензия

Код примеров, заготовок и решений объявлен в `Cargo.toml` под
MIT OR Apache-2.0. Лицензия текста книги пока не выбрана.
