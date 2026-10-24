# 10.4. Параметры-константы

## Значение в типе

Параметром может быть не только тип, но и значение, известное при
компиляции. Длина массива `[T; N]` — такой параметр с первых версий
языка, а с Rust 1.51 их можно объявлять самим:

```rust
{{#include ../../code/ch10/src/const_generic.rs:wrap}}
```

`const N: usize` — *параметр-константа* (const generic). `wrap::<16>` и
`wrap::<10>` — две разные функции, и в каждой делитель — литерал. Это
видно по коду:

```console
$ cargo asm -p ch10 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch10_wrap16
ch10_wrap16:
	push {r7, lr}
	mov r7, sp
	and r0, r0, #15
	pop {r7, pc}
$ cargo asm -p ch10 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch10_wrap10
ch10_wrap10:
	push {r7, lr}
	mov r7, sp
	movw r1, #52429
	movt r1, #52428
	umull r1, r2, r0, r1
	lsrs r1, r2, #3
	add.w r1, r1, r1, lsl #2
	sub.w r0, r0, r1, lsl #1
	pop {r7, pc}
$ cargo asm -p ch10 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch10_wrap_dyn
ch10_wrap_dyn:
	cmp r1, #0
	ittt ne
	udivne r2, r0, r1
	mlsne r0, r2, r1, r0
	bxne lr
	push {r7, lr}
	mov r7, sp
	movw r0, :lower16:.Lanon.dee5ae641167e8a8992033d3da56586c.1
	movt r0, :upper16:.Lanon.dee5ae641167e8a8992033d3da56586c.1
	bl core::panicking::panic_const::panic_const_rem_by_zero
```

Остаток от деления на 16 — маска `and #15`. На 10 — умножение на
0xCCCCCCCD с выборкой старшего слова `umull`: частное `i / 10` без
деления, затем `i - 10 · (i / 10)` двумя инструкциями со сдвигом
операнда. Делитель
во время работы требует `udiv`, от 2 до 12 тактов на Cortex-M3, и
проверки на ноль с паникой. Ни в одной из двух копий `wrap` проверки на
ноль нет: компилятор знает, что ни 16, ни 10 не ноль. На x86-64 картина
та же: `and eax, 15`, умножение на магическую константу `mul rcx` и
`div` для делителя из регистра.

На Cortex-M0 аппаратного деления нет вовсе, и выигрыш другой:

```console
$ cargo asm -p ch10 --lib --simplify --no-default-features --target thumbv6m-none-eabi ch10_wrap10
ch10_wrap10:
	push {r7, lr}
	add r7, sp, #0
	movs r1, #10
	bl __aeabi_uidivmod
	mov r0, r1
	pop {r7, pc}
```

Умножения 32 × 32 → 64 у M0 тоже нет, и трюк с магической константой
не окупается: LLVM вызывает программное деление из `compiler_builtins`,
десятки тактов. А `wrap::<16>` и там остаётся одной маской. Отсюда
правило для прошивок: ёмкость кольцевых буферов — степень двойки, и
лучше, чтобы это проверял компилятор.

## Ёмкость буфера как часть типа

Кольцевой буфер последних `N` отсчётов:

```rust
{{#include ../../code/ch10/src/const_generic.rs:ring}}
```

`Ring<i16, 64>` и `Ring<i16, 32>` — разные типы, и место под буфер
известно при компиляции: `size_of::<Ring<u8>>()` — 16 байт данных и два
`usize`, без кучи и без указателя на внешний массив. `N` по умолчанию
16, поэтому `Ring<u8>` — то же, что `Ring<u8, 16>`. Константа
`CAPACITY` связывает параметр со значением, которое видно снаружи, а
индексы `wrap::<N>` внутри методов в каждой копии превращаются в маску.

Блок `const { … }` внутри `new` вычисляется при компиляции, отдельно для
каждого `N`, с которым `Ring` создают. Ёмкость 10 не пройдёт:

```text
error[E0080]: evaluation panicked: ёмкость кольца — степень двойки
 --> /rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library/core/src/panic.rs:62:8
  |
  = note: evaluation of `ch10::const_generic::Ring::<u8, 10>::new::{constant#0}` failed here
…
note: the above error was encountered while instantiating `fn Ring::<u8, 10>::new`
 --> e0080.rs:4:13
  |
4 |     let r = Ring::<u8, 10>::new();
  |             ^^^^^^^^^^^^^^^^^^^^^
```

<div class="box trap">

**Ловушка.** Ошибка возникает при мономорфизации, то есть при создании
машинного кода. `cargo check` машинного кода не создаёт и
`Ring::<u8, 10>::new()` пропускает молча: та же программа проходит
`check` и падает на `build`. Проверки `const { assert!(…) }` над
параметрами ловит только настоящая сборка: `cargo build`, `cargo test`
или `cargo clippy`, который проверяет, но не мономорфизует, их тоже
пропустит. В CI должна быть хотя бы одна сборка.

</div>

## Чего пока нельзя

Параметр-константа может стоять в типе только целиком. Выражение над
ним — нестабильная возможность `generic_const_exprs`:

```text
error: generic parameters may not be used in const operations
 --> twice.rs:1:50
  |
1 | pub fn twice<const N: usize>(a: [u8; N]) -> [u8; N * 2] {
  |                                                  ^ cannot perform const operation using `N`
  |
  = help: const parameters may only be used as standalone arguments here, i.e. `N`
```

Функция, склеивающая два массива, поэтому не может написать в
результате `[u8; A + B]`. Обходной путь — третий параметр, который
вывод типов берёт у вызывающего, и проверка равенства при мономорфизации:

```rust
{{#include ../../code/ch10/src/const_generic.rs:concat}}
```

`let frame: [u8; 5] = concat([0xA5, 0x01], [1, 2, 3]);` выводит `A = 2`,
`B = 3` из аргументов и `S = 5` из аннотации. С аннотацией `[u8; 4]`
сборка падает с E0080 «длина результата должна быть A + B» — так же, как
`Ring` с ёмкостью 10.

Тип параметра — только целое, `bool` или `char`:

```text
error: `&'static str` is forbidden as the type of a const generic parameter
 --> tag.rs:1:28
  |
1 | pub struct Tag<const NAME: &'static str>;
  |                            ^^^^^^^^^^^^
  |
  = note: the only supported types are integers, `bool`, and `char`
```

Структуры, перечисления и ссылки в параметрах — нестабильные
`min_adt_const_params` и `unsized_const_params`, их и предлагает
nightly. Выражение в аргументе, а не в объявлении, писать можно, но в
фигурных скобках: `wrap::<{ 1 << 4 }>(17)`. Без скобок rustc отвечает
«expressions must be enclosed in braces to be used as const generic
arguments»: внутри `::<…>` знаки `<` и `>` иначе были бы
неоднозначны.

## Таблицы при компиляции

Параметр-константа и `const fn` вместе строят таблицы во flash:

```rust
{{#include ../../code/ch10/src/const_generic.rs:from-fn}}
```

Длину таблицы `squares` берёт из типа `static SQUARES: [u16; 16]`, а сама
таблица вычисляется при компиляции и лежит в `.rodata`: кода, который её
заполняет, в прошивке нет. Тело `const fn` пока ограничено: циклы `for`
по итераторам в нём недоступны, поэтому цикл записан через `while`. Для
таблиц, которые строятся при запуске, есть `core::array::from_fn(|i|
…)`, а `[1u8, 2, 3].map(u16::from)` преобразует массив, не теряя длину в
типе. Вычисления при компиляции подробно разбирает [глава 25](../ch25/index.md).

<div class="box c">

**Как в C.** Размер массива в параметре функции C — украшение:
`void f(int a[16])` значит `int *a`, и `sizeof a` внутри даёт размер
указателя. Ёмкость буфера задаётся `#define RING_N 16`, одна на всю
программу, а два буфера разной ёмкости требуют двух копий кода, обычно
через макрос. В Rust `[T; N]` и `Ring<T, N>` несут длину в типе, функция
`fn f(a: &[u8; 16])` не примет массив из 15 байт, а копии кода для
разных `N` компилятор создаёт сам.

</div>

<div class="box check">

**Проверь себя.** Сколько байт займёт `Ring<u32, 64>` на Cortex-M3 и на
x86-64? Что выдаст `cargo check` для `concat::<2, 3, 4>`, а что `cargo
build`? Почему в `ch10_wrap10` нет проверки деления на ноль, а в
`ch10_wrap_dyn` есть, и что будет на M0 с `ch10_wrap_dyn`?

</div>
