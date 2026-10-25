# 11.1. dyn Trait и толстые указатели

## Объект трейта

Датчик в этой главе почти тот же, что в главе 9, но `read` берёт
`&self`: объекты будут лежать и во flash, где менять нечего.

```rust
{{#include ../../code/ch11/src/fat.rs:sensor}}
```

Тип `dyn Sensor` значит «какой-то тип, который реализует `Sensor`».
Какой именно, при компиляции неизвестно, а значит, неизвестен и размер:
за `dyn Sensor` может стоять двухбайтовый `Thermo` или четырёхбайтовый
`Counter`. Положить такое значение в переменную нельзя:

```text
error[E0277]: the size for values of type `dyn Sensor` cannot be known at compilation time
 --> e1.rs:3:9
  |
3 |     let s: dyn Sensor = Thermo { raw: 344 };
  |         ^ doesn't have a size known at compile-time
  |
  = help: the trait `Sized` is not implemented for `dyn Sensor`
  = note: all local variables must have a statically known size
```

Значение без размера живёт только за указателем: `&dyn Sensor`, `&mut
dyn Sensor`, `Box<dyn Sensor>`. Такой указатель называют *объектом
трейта* (trait object). Получают его *приведением без размера* (unsizing
coercion) из указателя на конкретный тип:

```rust
{{#include ../../code/ch11/src/fat.rs:coerce}}
```

В [срезе][g-slice] лежат указатели одного типа, `&dyn Sensor`, хотя смотрят они на
значения разных типов. Приведение компилятор делает сам там, где тип
результата известен: в аргументе функции, в `let` с аннотацией, в
литерале массива, как здесь, или явно через `as &dyn Sensor`. `demo`
возвращает 2157: 2150 сотых градуса от термодатчика и 7 импульсов от
счётчика.

## Толстый указатель

Чтобы вызвать `read` у значения неизвестного типа, нужно знать, какую
функцию звать. Это знание едет вместе с указателем: `&dyn Sensor`
состоит из двух слов — адреса данных и адреса *таблицы методов*
(vtable), которую компилятор построил для пары `Thermo` и `Sensor`.

```rust
{{#include ../../code/ch11/src/fat.rs:sizes}}
```

`sizes()` возвращает `([1, 2, 2, 2], 2, 4)` и на x86-64, и на Cortex-M:
`&Thermo` — одно слово, `&dyn Sensor` и `*const dyn Sensor` — два, как и
срез `&[i16]`, у которого второе слово — длина. Такие указатели называют
*толстыми* (wide, fat pointer), обычные — *тонкими*. `size_of_val`
узнаёт размер значения во время работы: у среза — из длины, у объекта
трейта — из таблицы методов.

<div class="box c">

**Как в C.** В `fops.c` указатель на таблицу лежит в каждом объекте, в
поле `ops`, и `struct thermo` из-за него занимает 24 байта. В Rust
таблица едет в указателе: `Thermo` остаётся двумя байтами, а платит
каждая ссылка на объект — второе слово. Так же устроены интерфейсы в Go;
в C++ указатель на таблицу виртуальных функций (vptr) лежит в объекте,
как в ядре Linux. Выбор Rust позволяет сделать объектом трейта значение
любого типа, даже чужого и даже `u8`, не меняя его [раскладки][g-layout].

</div>

## Вызов через таблицу

```rust
{{#include ../../code/ch11/src/fat.rs:call}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_read_dyn
ch11_read_dyn:
	jmp qword ptr [rsi + 24]
$ cargo asm -p ch11 --lib --simplify ch11_read_thermo
ch11_read_thermo:
	movsx eax, word ptr [rdi]
	imul ecx, eax, 100
	lea eax, [rcx + 15]
	test ecx, ecx
	cmovns eax, ecx
	sar eax, 4
	ret
```

Толстый указатель передаётся в двух регистрах: адрес данных в `rdi`,
адрес таблицы в `rsi`. Вызов `read` — переход по слову таблицы со
смещением 24, четвёртому по счёту: первые три заняты деструктором,
размером и выравниванием (раздел [11.2](02-vtable.md)). Адрес данных
уже лежит в `rdi`, первом аргументе, и `read` получает его как `&self`.
`call` не нужен: результат `read` и есть результат функции, и LLVM
заменил вызов переходом. Для известного типа вызова нет вовсе: тело
`read` встроено, деление на 16 со знаком — `sar` с поправкой для
отрицательных.

На Cortex-M3 и Cortex-M0 то же самое:

```console
$ cargo asm -p ch11 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch11_read_dyn
ch11_read_dyn:
	push {r7, lr}
	mov r7, sp
	ldr r1, [r1, #12]
	pop.w {r7, lr}
	bx r1
$ cargo asm -p ch11 --lib --simplify --no-default-features --target thumbv6m-none-eabi ch11_read_dyn
ch11_read_dyn:
	push {r7, lr}
	add r7, sp, #0
	ldr r1, [r1, #12]
	blx r1
	pop {r7, pc}
```

Данные в `r0`, таблица в `r1`, слово 3 таблицы — смещение 12 при
четырёхбайтовых словах. M3 тоже обошёлся переходом `bx r1`, а M0 зовёт
`read` через `blx` и возвращается сам: в [ARMv6-M][g-armv7m] нет 32-битной
`pop.w`, которой M3 одной инструкцией снимает кадр вместе с `lr`, и
LLVM не стал заменять вызов переходом. `push {r7, lr}` и `mov r7, sp` —
кадр с указателем в `r7`, который цели Cortex-M ведут в каждой функции
(`frame-pointer: always`, раздел [0.1](../ch00/01-rustup.md)).

## Время жизни объекта

За `dyn Sensor` может стоять тип, который держит заём. Поэтому у
объекта трейта есть [время жизни][g-lifetime], и его можно не писать только потому,
что у него есть правила по умолчанию:

| Запись | Что значит |
|---|---|
| `Box<dyn Sensor>` | `Box<dyn Sensor + 'static>` |
| `&'a dyn Sensor` | `&'a (dyn Sensor + 'a)` |
| `&'a mut dyn Sensor` | `&'a mut (dyn Sensor + 'a)` |

Датчик поверх чужого буфера держит заём:

```rust
{{#include ../../code/ch11/src/fat.rs:window}}
```

В `&dyn Sensor` он помещается: объект живёт не меньше ссылки, этого и
требует правило. А в `Box<dyn Sensor>` — нет, там ожидается `'static`:

```text
error: lifetime may not live long enough
 --> e2.rs:3:5
  |
2 | fn boxed(buf: &[i16]) -> Box<dyn Sensor> {
  |               - let's call the lifetime of this reference `'1`
3 |     Box::new(Window { buf })
  |     ^^^^^^^^^^^^^^^^^^^^^^^^ returning this value requires that `'1` must outlive `'static`
  |
help: to declare that the trait object captures data from argument `buf`, you can add an explicit `'_` lifetime bound
  |
2 | fn boxed(buf: &[i16]) -> Box<dyn Sensor + '_> {
  |                                         ++++
```

С `Box<dyn Sensor + 'a>` функция собирается: [doc-тест][g-doctest] в
`code/ch11/src/fat.rs` проверяет оба варианта.

## Автотрейты в типе объекта

`dyn Sensor` ничего не обещает о потоках: за ним может стоять тип с
`Cell` или `Rc` внутри. Передать такой объект в другой поток нельзя:

```text
error[E0277]: `dyn Sensor` cannot be sent between threads safely
 --> e3.rs:3:24
  |
3 |     std::thread::spawn(move || s.read());
  |     ------------------ ^^^^^^^^^^^^^^^^ `dyn Sensor` cannot be sent between threads safely
  |     |
  |     required by a bound introduced by this call
  |
  = help: the trait `Send` is not implemented for `dyn Sensor`
```

Обещание дописывают к типу объекта: `Box<dyn Sensor + Send>`, `&(dyn
Sensor + Sync)`. Через `+` к трейту добавляются только [автотрейты][g-auto-trait] из
раздела [9.5](../ch09/05-markers.md) и время жизни: `dyn Sensor + Debug`
не бывает, для этого нужен свой трейт с обоими супертрейтами. Таблица
у `dyn Sensor + Send` та же, что у `dyn Sensor`: у автотрейтов нет
методов, а `&(dyn Sensor + Send)` приводится к `&dyn Sensor`, не меняя
ни одного из двух слов.

<div class="box trap">

**Ловушка.** `dyn Trait` — не тип «интерфейс», как в Java или C#.
`Vec<dyn Sensor>` не собирается: элементы вектора должны быть одного
размера. Нужен `Vec<Box<dyn Sensor>>` — вектор толстых указателей и
отдельная аллокация на каждый датчик, или `Vec<&dyn Sensor>`, если
датчики живут где-то ещё. Если все типы известны заранее, перечисление
из раздела [11.7](07-closed.md) обходится без того и другого.

</div>

<div class="box check">

**Проверь себя.** Почему `jmp qword ptr [rsi + 24]` достаточно, чтобы
`read` получил `&self`, хотя функция ничего не делает с `rdi`? Сколько
байт займёт `[&dyn Sensor; 8]` на Cortex-M3 и сколько — `[&Thermo; 8]`?
Что напечатает `size_of_val` для `&dyn Sensor`, полученного из
`&Window`, и зависит ли ответ от длины буфера?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-armv7m]: ../appendix-f.md#armv7m "ARMv6-M, ARMv7-M, ARMv7E-M: Версии архитектуры ARM для микроконтроллеров."
[g-auto-trait]: ../appendix-f.md#auto-trait "Автотрейт (auto trait): Маркерный трейт, который компилятор реализует сам для каждого типа, все поля которого его реализуют: Send, Sync, Unpin, UnwindSafe."
[g-doctest]: ../appendix-f.md#doctest "doc-тест (doctest): Пример кода в документирующем комментарии ///, который cargo test собирает и запускает как тест."
[g-layout]: ../appendix-f.md#layout "Раскладка (layout): Размер, выравнивание и смещения полей типа в памяти."
[g-lifetime]: ../appendix-f.md#lifetime "Время жизни (lifetime): Имя для участка программы, на котором ссылка действительна: 'a в &'a str."
[g-slice]: ../appendix-f.md#slice "Срез (slice): Непрерывная последовательность элементов, длина которой известна только во время выполнения: [T], а str — срез байт UTF-8."
