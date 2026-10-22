# 4.7. Задачи

Заготовки лежат в `exercises/ch04`, тесты, общие для заготовок и решений,
лежат в `tests-shared/ch04.rs`. Проверка:

```console
$ cargo test -p ch04-exercises            # все задачи
$ cargo test -p ch04-exercises task_c     # одна задача
```

Задачи A, B и D запускают компиляторы: rustc из `rust-toolchain.toml`,
nightly для Polonius (без него эта часть D пропускается с сообщением),
clang или arm-none-eabi-gcc и `cc` хоста. Пока задачи не решены, тесты
падают на `todo!()` или на пустых ответах. Разборы решений в
[приложении E](../appendix-e/ch04.md), но сначала попробуйте сами.

## A. Что скажет проверка

В `code/ch04/fragments` лежат десять фрагментов. Для каждого до запуска
компилятора решите: компилируется ли он, а если нет — какой код у первой
ошибки: E0499 (два писателя), E0502 (читатель и писатель) или E0506
(присваивание заимствованному).

**a01.rs**

```rust
{{#include ../../code/ch04/fragments/a01.rs}}
```

**a02.rs**

```rust
{{#include ../../code/ch04/fragments/a02.rs}}
```

**a03.rs**

```rust
{{#include ../../code/ch04/fragments/a03.rs}}
```

**a04.rs**

```rust
{{#include ../../code/ch04/fragments/a04.rs}}
```

**a05.rs**

```rust
{{#include ../../code/ch04/fragments/a05.rs}}
```

**a06.rs**

```rust
{{#include ../../code/ch04/fragments/a06.rs}}
```

**a07.rs**

```rust
{{#include ../../code/ch04/fragments/a07.rs}}
```

**a08.rs**

```rust
{{#include ../../code/ch04/fragments/a08.rs}}
```

**a09.rs**

```rust
{{#include ../../code/ch04/fragments/a09.rs}}
```

**a10.rs**

```rust
{{#include ../../code/ch04/fragments/a10.rs}}
```

Ответы запишите в `exercises/ch04/src/a_borrowck.rs`: `"ok"` или код
ошибки, и к каждому — комментарий с правилом из разделов 4.1–4.3, которое
решает дело. Тест соберёт каждый фрагмент rustc и покажет расхождения:

```text
a01.rs: ваш ответ E0499, на самом деле ok
```

Хотя бы два ответа почти наверняка окажутся неожиданными. Для каждого
найдите, в каком порядке компилятор вычисляет выражение, и запишите
этот порядок в комментарии.

## B. Лишняя загрузка

В `exercises/ch04/src/b_add_twice.rs` напишите

```rust
pub fn ch04b_add_twice(a: &mut i32, b: &i32)
```

которая дважды прибавляет `*b` к `*a`, а в `exercises/ch04/c/add_twice.c` —
её аналоги на C: `add_twice(int *a, const int *b)` и
`add_twice_restrict` с `restrict` у обоих параметров. Тела одинаковые:
`*a += *b; *a += *b;`.

Соберите все три для Cortex-M3 и посмотрите ассемблер:

```console
$ cargo asm -p ch04-exercises --lib --target thumbv7m-none-eabi ch04b_add_twice
$ clang -O2 --target=thumbv7m-none-eabi -mcpu=cortex-m3 -S -o - exercises/ch04/c/add_twice.c
```

Запишите в `LDR` и `STR` число инструкций `ldr` и `str` в каждой функции
и найдите загрузку, которая есть только в C без `restrict`. Объясните, в
каком случае она нужна. Затем ответьте, что окажется в `x` после
`int x = 1; add_twice(&x, &x);` и после того же вызова
`add_twice_restrict` (сборка `cc -O2` на хосте), и запишите в `ALIASED`.
Тест соберёт ваш C-файл с небольшой `main` и сверит. Почему такой вызов
нельзя написать для `ch04b_add_twice` и какой код ошибки выдаст
компилятор?

## C. Граф сборки

Система сборки прошивки знает правила вида «`firmware.elf` зависит от
`main.o`, `hal.o` и `link.x`». В `exercises/ch04/src/c_build_graph.rs`
реализуйте граф задач:

```rust
let mut g = Graph::new();
let elf = g.task("firmware.elf");     // номер задачи, новая — добавляется
let main = g.task("main.o");
g.depends(elf, main);                 // elf нельзя начинать до main
match g.build_order() {
    Ok(order) => {}                   // порядок: каждая задача после своих зависимостей
    Err(cycle) => {}                  // цикл: каждая зависит от следующей, последняя — от первой
}
```

Требования:

1. Узлы ссылаются друг на друга номерами `TaskId`, а не ссылками. Ни
   `unsafe`, ни `Rc`, ни `RefCell`: тест `arena_only` проверяет исходный
   текст.
2. Цикл возвращается целиком и только сам цикл: задачи, которые от него
   зависят, в ответ не входят.
3. Обход без рекурсии. Тест `long_chain` строит цепочку из 100 000 задач,
   и рекурсивный обход в глубину переполнит стек тестового потока
   (2 МиБ). Посчитайте, сколько байт стека занимал бы один уровень
   вашей рекурсии.
4. Сложность — линейная от числа задач и рёбер.

Тест `finds_cycle` строит правдоподобную ошибку в правилах: `gen.h`
генерирует программа `gen`, которая сама включает `gen.h`.

## D. Polonius

Задача про две корректные программы, которые rustc 1.97 отвергает.

Первая — выдача слота из пула буферов:

```rust
pub fn acquire(pool: &mut Vec<Slot>) -> &mut Slot
```

Функция возвращает первый свободный слот, пометив его занятым, а если
свободных нет — добавляет новый в конец пула. Напишите её в
`exercises/ch04/polonius/acquire.rs` так, как написали бы на C: цикл по
`iter_mut` с ранним `return` и `push` после цикла. Тест проверит, что
закреплённый rustc её отвергает ошибкой заимствования, а nightly с
`-Zpolonius=next` принимает. В `exercises/ch04/src/d_polonius.rs`
напишите версию, которую принимает stable, и объясните, чем она
платит.

Вторая — добавление в конец односвязного списка:

```rust
pub fn push_back(head: &mut Node, val: u32)
```

Напишите в `exercises/ch04/polonius/push_back.rs` версию с курсором:
`while let Some(next) = &mut cur.next { cur = next; }`, затем вставка.
Тест проверит, что её отвергает и stable, и Polonius в режиме
`-Zpolonius=next`. Объясните, чем этот случай отличается от первого, и
напишите в `d_polonius.rs` версию, которую принимает stable.

Для отчёта соберите обе программы ещё и с `-Zpolonius=legacy`. Тест
кладёт обёртки с определениями `Slot` и `Node` в
`target/tmp/ch04-exercises-d/`, их можно собрать руками:

```console
$ rustup run nightly rustc --edition 2024 --crate-type lib --emit metadata \
    -Zpolonius=legacy -o /tmp/d.rmeta target/tmp/ch04-exercises-d/push_back.rs
```

Что изменилось и почему, по-вашему, в компилятор идёт не этот вариант?
