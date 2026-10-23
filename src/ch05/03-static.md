# 5.3. `'static` и стирание времён жизни

## Данные на всё время работы программы

`'static` — время жизни, которое длится до конца программы. Его имеют
ссылки на данные, которые не освобождаются никогда:

```rust
{{#include ../../code/ch05/src/statics.rs:data}}
```

- Строковый литерал имеет тип `&'static str`. Байты строки компилятор
  кладёт в `.rodata` исполняемого файла, и ссылка на них действительна
  всегда.
- `static` — переменная с одним адресом на всю программу. Ссылка на
  неё — тоже `&'static`.
- В `gains` ссылка на массив-литерал получает `'static`, хотя массив
  записан внутри функции. Выражение `[1, 2, 4, 8]` константное, и
  компилятор кладёт его в статическую память, а не на стек: это
  *продвижение констант* (constant promotion). Тест проверяет, что два
  вызова `gains()` возвращают один и тот же адрес.

Продвигается только то, что можно вычислить при компиляции. Массив со
значением из аргумента остаётся временным на стеке:

```text
error[E0515]: cannot return reference to temporary value
 --> statics_1.rs:4:5
  |
4 |     &[n, 2]
  |     ^------
  |     ||
  |     |temporary value created here
  |     returns a reference to data owned by the current function
```

## Где это лежит на микроконтроллере

На Cortex-M `.rodata` — это flash. Прошивка `firmware/ch05-addr`
печатает адреса разных видов данных `'static` и по карте памяти из
`memory.x` (flash с адреса 0, ОЗУ с `0x2000_0000`) подписывает регион:

```rust,ignore
{{#include ../../firmware/ch05-addr/src/main.rs:statics}}
```

```rust,ignore
{{#include ../../firmware/ch05-addr/src/main.rs:main}}
```

Вывод в QEMU на Cortex-M3 (LM3S6965):

```console
$ cd firmware && cargo run --release -p ch05-addr
байты BANNER             0x00001876  flash
static BANNER (&str)     0x0000189c  flash
static LIMITS            0x000018a4  flash
&GAINS (const)           0x000017d2  flash
static BOOTS (.bss)      0x20000004  ОЗУ
static SERIAL (.data)    0x20000000  ОЗУ
singleton! буфер         0x20000010  ОЗУ
локальный массив         0x20003fa8  ОЗУ
BOOTS = 1, SERIAL = 0x5eed
```

- Байты строки, сама переменная `BANNER` (указатель и длина, 8 байт),
  таблица `LIMITS` и продвинутая константа `GAINS` лежат во flash.
  Записать туда программа не может, и неизменяемым данным это не нужно.
- `BOOTS` и `SERIAL` — тоже `static`, но атомарные: их меняют через `&`,
  и место им только в ОЗУ. Компилятор кладёт в ОЗУ любой `static` с
  внутренней изменяемостью (`UnsafeCell` внутри, глава
  [14](../ch14/05-cells.md)), даже без `mut`. Нулевой `BOOTS` попадает в `.bss`,
  который код сброса обнуляет, а ненулевой `SERIAL` — в `.data`: его
  начальное значение хранится во flash и копируется в ОЗУ до `main`
  (задача [0.D](../ch00/07-tasks.md) проходит этот путь в gdb).
- Буфер из `cortex_m::singleton!` — `&'static mut [u8; 64]` в ОЗУ.
  Макрос выдаёт ссылку один раз (второй вызов вернёт `None`) и для этого
  берёт критическую секцию. `'static` здесь значит «до выключения», а не
  «во flash».
- Локальный массив — на стеке, у верхней границы ОЗУ. Ссылка на него
  `'static` не бывает.

На Cortex-M0 (micro:bit) адреса во flash другие — другой набор
инструкций и другой размер кода, — а в ОЗУ те же. `run-demos.sh`
проверяет обе машины.

<div class="box ref">

**Справка: секции.** `.rodata`, `.data` и `.bss` — имена секций в
объектном файле, общие для GCC, clang и rustc. На хосте под Linux
`.rodata` отображается в память только для чтения, и запись туда
кончается `SIGSEGV`. На микроконтроллере без MMU данные защищает сама
flash: обычная инструкция `str` её не программирует, для записи есть
отдельная процедура через регистры контроллера flash. Какие секции куда
кладёт компоновщик, задаёт `link.x` из `cortex-m-rt` вместе с `memory.x`
проекта.

</div>

## `T: 'static` — свойство типа

Ограничение `T: 'static` встречается в `std::thread::spawn` и в
`Box<dyn Any>`. Читают его часто как «значение
живёт вечно». Это не так: оно значит, что в *типе* `T` нет ссылок
короче `'static`.

```rust
{{#include ../../code/ch05/src/statics.rs:bound}}
```

`String`, `u32`, `Vec<u8>` — типы без ссылок, им `'static`
удовлетворяет. Значение такого типа можно передать в `keep` и уничтожить
через строчку. Тест `static_bound_is_not_forever` кладёт в `Settings`
значение, которое при уничтожении взводит флаг, и проверяет, что флаг
взведён после `drop(s)`.

Ссылке на локальную строку `keep` откажет, даже если строка живёт до
конца программы:

```text
error[E0597]: `local` does not live long enough
 --> statics_0.rs:6:14
  |
5 | let local = String::from("uart1");
  |     ----- binding `local` declared here
6 | let r = keep(&local);
  |         -----^^^^^^-
  |         |    |
  |         |    borrowed value does not live long enough
  |         argument requires that `local` is borrowed for `'static`
7 | println!("{r}");
8 | }
  | - `local` dropped here while still borrowed
```

Зачем такое ограничение `dyn Any`: `Settings::get` достаёт значение по
типу, а типы, которые различаются только временами жизни, во время
выполнения неразличимы (ниже). Хранилище, в которое можно положить
`&'a str`, а достать `&'static str`, было бы дырой.

У `Box<dyn Trait>` без указанного времени жизни есть неявное `+ 'static`.
Обобщённый `T` в него просто так не кладётся:

```text
error[E0310]: the parameter type `T` may not live long enough
 --> statics_2.rs:6:5
  |
5 | pub fn boxed<T: Display>(t: T) -> Box<dyn Display> {
  |                                       ----------- this `dyn Trait` has an implicit `'static` lifetime bound
6 |     Box::new(t)
  |     ^^^^^^^^^^^
  |     |
  |     the parameter type `T` must be valid for the static lifetime...
  |     ...so that the type `T` will meet its required lifetime bounds
```

Компилятор предлагает `T: 'static`. Если объект должен уметь хранить
ссылки, лучше назвать его время жизни, как `boxed<'a, T: Display + 'a>`
выше: тест кладёт в такой `Box` ссылку на локальный `String`.

## `Box::leak`: `'static` во время выполнения

Иногда значение становится известно только при запуске, а нужно до
выключения: имя станции из конфигурации, таблица калибровки, прочитанная
из EEPROM. `Box::leak` превращает владение в `&'static mut`, отказываясь
от освобождения:

```rust
{{#include ../../code/ch05/src/statics.rs:leak}}
```

Память под имя не вернётся никогда. Один раз при старте это нормально,
так устроен `singleton!`, только у него память статическая. Вызов
`leak_name` в цикле — утечка. Тест освобождает память вручную через
`Box::from_raw`, иначе Miri сообщит об утечке. `into_boxed_str` здесь не
случаен: он сжимает буфер до длины строки, и `Box<str>` потом
освобождается с тем же размером, с каким выделен. `String::leak` буфер
не сжимает.

## Стирание

Время жизни существует только при проверке. В машинном коде его нет:

```rust
{{#include ../../code/ch05/src/erase.rs:erase}}
```

```text
$ cargo asm -p ch05 --lib ch05_first 0
ch05_first:
	mov rax, rdi
	test rsi, rsi
	cmove rax, rsi
	ret
```

Срез приходит в `rdi` (указатель) и `rsi` (длина). `Option<&u8>` — это
указатель, где `None` — ноль: при пустом срезе `cmove` кладёт в `rax`
нулевую длину. На Cortex-M3 то же самое в `r0`, `r1`:

```text
$ cargo asm -p ch05 --lib --target thumbv7m-none-eabi ch05_first 0
ch05_first:
	push {r7, lr}
	mov r7, sp
	cmp r1, #0
	it eq
	moveq r0, r1
	pop {r7, pc}
```

`ch05_first_static` cargo asm не находит: своего тела у неё нет. Код
двух функций совпал, и проход LLVM MergeFunctions оставил одно тело, а
второе имя сделал псевдонимом:

```console
$ cargo rustc -p ch05 --release --lib -- --emit=llvm-ir
$ grep '^@ch05_first_static' target/release/deps/ch05-*.ll
@ch05_first_static = unnamed_addr alias ptr (ptr, i64), ptr @ch05_first
```

Это проверяет тест `code/ch05/tests/erase.rs`. Даже имя типа во время
выполнения не помнит времени жизни: `type_name::<Tlv<'static>>()` — это
`"ch05::tlv::Tlv<'_>"`, а `type_name::<&'static str>()` — `"&str"`.

Отсюда два следствия. Проверка времён жизни ничего не стоит во время
выполнения: ни флагов, ни счётчиков. И во время выполнения нельзя
спросить, `'static` ли ссылка, и выбрать по ответу разную реализацию:
обобщённый код компилируется один раз для всех времён жизни.

<div class="box c">

**Как в C.** `static const` в C тоже ложится во flash, а глобальная
переменная без `const` — в `.data` или `.bss`. Разница в том, что в C
указатель на строковый литерал имеет тип `char *` (не `const`), и запись
через него компилируется — и падает на хосте или не меняет данные во
flash. В Rust `&'static str` неизменяем по типу.

</div>

<div class="box check">

**Проверь себя.** Удовлетворяет ли `&'static str` ограничению
`T: 'static`? А `Vec<&'a u8>`? Почему `thread::spawn` требует
`F: 'static`, хотя поток может завершиться раньше `main`?

</div>
