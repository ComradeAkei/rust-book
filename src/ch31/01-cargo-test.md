# 31.1. Тесты в cargo

<div class="box ref">

**Справка: что такое тест для cargo.** `cargo test` собирает крейт
второй раз, с флагом `--test` у rustc. Получается обычный исполняемый
файл, в котором вместо вашего `main` — `main` из libtest, библиотеки
тестов в составе std. Функции с атрибутом `#[test]` компилятор собирает
в таблицу, libtest проходит её и запускает каждую функцию в отдельном
потоке: по умолчанию потоков столько, сколько ядер
(`--test-threads=N` или переменная `RUST_TEST_THREADS` меняют число).
Тест прошёл, если функция вернулась, и не прошёл, если она
запаниковала, — паника в потоке теста не роняет процесс. Вывод теста
(`println!`) libtest перехватывает и показывает только у упавших;
`--nocapture` выключает перехват. Код выхода — 0, если все прошли, и
101, если нет: по нему вердикт видят cargo, CI и `make`.

</div>

## Модульные тесты

Пример главы — скользящее окно последних `N` измерений датчика со
средним за O(1). Сумма хранится рядом с буфером, и при вытеснении
старое значение из неё вычитается:

```rust,ignore
{{#include ../../code/ch31/src/basics.rs:window}}
```

*Модульные тесты* (unit tests) живут в том же файле, в [модуле][g-module] под
`#[cfg(test)]`: в обычную сборку он не попадает. Модуль — потомок
`basics`, поэтому видит его приватные поля и функции. Так проверяют
инвариант, который снаружи не наблюдаем: сумма равна сумме содержимого
буфера.

```rust,ignore
{{#include ../../code/ch31/src/basics.rs:unit}}
```

Три формы теста. Обычный — функция без аргументов, которая паникует
через `assert!` или `assert_eq!`, если что-то не так. `#[should_panic]`
— наоборот, тест проходит только при панике, а `expected` требует
подстроку в её сообщении: без него тест прошёл бы и от паники совсем по
другой причине. Тест, который возвращает `Result`, может пользоваться
`?`: ошибка — провал теста.

```console
$ cargo test -p ch31 --lib basics
running 4 tests
test basics::tests::parse_and_display ... ok
test basics::tests::mean_of_extremes_fits ... ok
test basics::tests::sum_follows_contents ... ok
test basics::tests::latest_beyond_window_panics - should panic ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.07s
```

Аргумент после `--lib` — фильтр: подстрока полного имени теста, и
восемь тестов других модулей отфильтрованы. Порядок строк от прогона к
прогону разный: тесты идут параллельно.

## Интеграционные тесты

Каждый файл прямо в каталоге `tests/` — *интеграционный тест* (integration
test), отдельный крейт, который зависит от библиотеки как обычный
пользователь и видит только её публичный API:

```rust,ignore
{{#include ../../code/ch31/tests/window.rs:integration}}
```

Общие помощники лежат в `tests/common/mod.rs`, а не в
`tests/common.rs`: файл в `tests/` cargo собрал бы отдельным тестовым
бинарником без единого теста, а подкаталог он не трогает.

```rust,ignore
{{#include ../../code/ch31/tests/common/mod.rs:common}}
```

У интеграционных тестов своя цена: каждый файл — отдельная компоновка,
и в большом проекте их сотня собирается заметно дольше одного бинарника
с сотней модулей. Поэтому тесты, которые не требуют отдельного процесса,
часто собирают в один `tests/integration/main.rs` с подмодулями.

## Тесты в документации

Пример в `///` — тоже тест. [rustdoc][g-rustdoc] вынимает блоки кода из комментариев,
оборачивает каждый в `fn main` и собирает как отдельный крейт,
зависимый от библиотеки. Пример из документации `Window` выше
выполняется при каждом `cargo test`, и если интерфейс поменяется, а
пример нет, тест упадёт. Пометки у блока меняют, что проверяется:
`should_panic` — пример должен запаниковать, `no_run` — собрать, но не
запускать, `ignore` — не собирать вовсе, `compile_fail` — пример *не*
должен собраться.

```console
$ cargo test -p ch31 --doc
running 2 tests
test code/ch31/src/basics.rs - basics::Window (line 10) ... ok
test code/ch31/src/basics.rs - basics::Window<N>::latest (line 75) - should panic ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

running 2 tests
test code/ch31/src/basics.rs - basics::SET_VAR_IS_UNSAFE (line 122) - compile fail ... ok
test code/ch31/src/basics.rs - basics::Window (line 26) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

all doctests ran in 0.35s; merged doctests compilation took 0.22s
```

`compile_fail` с кодом ошибки — способ закрепить тестом то, что
гарантирует компилятор. Окно на ноль измерений не собирается: константа
`NONEMPTY` вычисляется при компиляции, и `assert!` в ней — ошибка
компиляции, а не паника. Код `E0080` в пометке требует именно эту
ошибку: опечатка в примере, из-за которой он не собрался бы по другой
причине, тест провалит. Вот что видит пользователь, который всё же
напишет `Window::<0>::new()`:

```console
$ cargo build
error[E0080]: evaluation panicked: окно на ноль измерений
  --> /rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library/core/src/panic.rs:62:8
   |
   = note: evaluation of `ch31::basics::Window::<0>::NONEMPTY` failed here
   |
  ::: code/ch31/src/basics.rs:40:26
   |
40 |     const NONEMPTY: () = assert!(N > 0, "окно на ноль измерений");
   |                          ---------------------------------------- in this macro invocation

note: erroneous constant encountered
  --> code/ch31/src/basics.rs:43:18
   |
43 |         let () = Self::NONEMPTY;
   |                  ^^^^^^^^^^^^^^

note: the above error was encountered while instantiating `fn Window::<0>::new`
 --> src/main.rs:2:13
  |
2 |     let w = ch31::basics::Window::<0>::new();
  |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

<div class="box trap">

**Ловушка: `cargo check` этой ошибки не видит.** Константа зависит от
`N`, и вычисляется она, только когда компилятор порождает код для
конкретного `Window::<0>` — при сборке, а не при проверке типов. Тот же
`main.rs` под `cargo check` проходит без единого слова, под `cargo build`
— нет. Пометка `compile_fail` надёжна потому, что rustdoc собирает
пример целиком. Строка `let () = Self::NONEMPTY;` в `new` нужна по той же
причине: константу, которую никто не использует, компилятор не
вычисляет вовсе.

</div>

Второй пример `compile_fail` фиксирует правило [редакции][g-edition] 2024:
`std::env::set_var` стал `unsafe fn`.

```rust,ignore
{{#include ../../code/ch31/src/basics.rs:edition}}
```

Это прямо касается тестов. Тесты одного бинарника — потоки одного
процесса, а окружение, текущий каталог, обработчики сигналов и
открытые дескрипторы у процесса одни. Тест, который меняет переменную
окружения, может сломать соседа, читающего её в тот же момент, а glibc
при параллельных `setenv` и `getenv` вправе упасть. Такие тесты либо
запускают в отдельном процессе, как ниже, либо не пишут вовсе.

<div class="box hood">

**Под капотом: один бинарник на все примеры.** До редакции 2024 rustdoc
собирал каждый пример отдельным исполняемым файлом, и сотня примеров
означала сотню компоновок. В редакции 2024 примеры одного крейта
сливаются в один бинарник, каждый в своём модуле, — отсюда строка
«merged [doctests][g-doctest] compilation took 0.22s». Отдельно собираются только
примеры, которые слить нельзя: `compile_fail` (им нужна своя ошибка) и
примеры со своими `#![...]` атрибутами крейта. Поэтому в выводе два
блока `running 2 tests`.

</div>

## Как выглядит упавший тест

Два теста, которые падают намеренно. Первый проверяет неверное
ожидание о среднем, второй разбирает строку со словом:

```rust,ignore
{{#include ../../code/ch31/tests/failing.rs:failing}}
```

`#[ignore]` убирает их из обычного прогона, а `--ignored` запускает
только их:

```console
$ cargo test -p ch31 --test failing -- --ignored --test-threads=1
running 2 tests
test mean_rounds_to_nearest ... FAILED
test parses_words ... FAILED

failures:

---- mean_rounds_to_nearest stdout ----

thread 'mean_rounds_to_nearest' (15638) panicked at code/ch31/tests/failing.rs:14:5:
assertion `left == right` failed: окно 1,2
  left: Some(1)
 right: Some(2)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- parses_words stdout ----
Error: ParseIntError { kind: InvalidDigit }


failures:
    mean_rounds_to_nearest
    parses_words

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ch31 --test failing`
```

`assert_eq!` печатает обе стороны через `Debug` и сообщение после
них — `"окно {w}"` во втором аргументе. Тест, вернувший `Err`, печатает
ошибку через `Debug` и не показывает, где она возникла: `?` пробросил её
из глубины, а строки в отчёте нет. Для теста, который проверяет разбор,
`unwrap()` с номером строки часто полезнее `?`. Код выхода — 101.

Аргументы после `--` принадлежат libtest. Полезные: `--list` печатает
имена, ничего не запуская; фильтр — подстрока имени, `--exact` требует
имя целиком; `--skip` исключает; `--test-threads=1` запускает тесты по
одному, когда они делят что-то общее. Этот отчёт сверяет тест
`failure_report`: он запускает собственный бинарник
(`std::env::current_exe()`) с `--ignored` и ищет строки в выводе.

## Свой `main` вместо libtest

Иногда тестов — таблица: векторы из стандарта, тысяча строк входа и
ответа. Писать по `#[test]` на строку незачем. `harness = false` в
`Cargo.toml` отключает libtest для одного тестового бинарника, и тогда
у него обычный `main`:

```toml
[[test]]
name = "vectors"
harness = false
```

```rust,ignore
{{#include ../../code/ch31/tests/vectors.rs:harness}}
```

Таблица — текстовый файл рядом, `include_str!` встраивает его при
компиляции:

```text
{{#include ../../code/ch31/tests/vectors.txt}}
```

```console
$ cargo test -p ch31 --test vectors
     Running tests/vectors.rs (target/debug/deps/vectors-b71a8d0f90330fb4)
векторы окна: 7 совпали, 0 нет
```

cargo смотрит только на код выхода. Отчёт, фильтры, параллельность —
забота самой программы. Так же устроены замеры [criterion][g-criterion] и [divan][g-divan] из
главы 30: у `[[bench]]` там тоже `harness = false`, и `main` даёт
библиотека замеров.

<div class="box c">

**Как в C.** Ближе всего — CMocka или Unity: таблица функций-тестов,
макросы проверок, свой `main` в отдельном файле, сборка через
`add_test` в CMake или цель `check` в Makefile. Модульный тест на
`static` функцию в C пишут, включая `.c` файл целиком (`#include
"window.c"`) или снимая `static` под `#ifdef TEST`. Примеры в
комментариях Doxygen не компилируются никогда. Проверить, что
неправильный код *не* собирается, в C можно только скриптом сборки:
`_Static_assert` ловит ошибку, но тест на сам факт ошибки — уже
внешний.

</div>

<div class="box check">

**Проверь себя.** 1. Тест `latest_beyond_window_panics` проходит. Что
будет, если в `latest` поменять текст сообщения, и что — если убрать
`expected`? 2. Почему `sum_follows_contents` нельзя написать как
интеграционный тест? 3. Пример `compile_fail` без кода ошибки прошёл
бы и с опечаткой `Windw::<0>::new()`. Почему, и как пометка `E0080` это
исправляет?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-criterion]: ../appendix-f.md#criterion "criterion: Библиотека замеров для cargo bench: прогрев, сто выборок разной длины, время на вызов — наклон прямой по точкам «вызовов в выборке — время выборки», доверительный интервал бутстрепом, выбросы и сравнение с прошлым прогоном."
[g-divan]: ../appendix-f.md#divan "divan: Библиотека замеров, в которой замер объявляют атрибутом #[divan::bench] над функцией, а параметры пишут прямо в атрибуте."
[g-doctest]: ../appendix-f.md#doctest "doc-тест (doctest): Пример кода в документирующем комментарии ///, который cargo test собирает и запускает как тест."
[g-edition]: ../appendix-f.md#edition "Редакция (edition): Версия правил языка, которую выбирает каждый крейт: 2015, 2018, 2021, 2024."
[g-module]: ../appendix-f.md#module "Модуль (module): Пространство имён внутри крейта со своими правилами видимости: объявляется словом mod, с телом в скобках или в отдельном файле."
[g-rustdoc]: ../appendix-f.md#rustdoc "rustdoc: Генератор документации Rust: собирает HTML из комментариев /// (cargo doc) и запускает doc-тесты."
