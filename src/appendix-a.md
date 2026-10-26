# Приложение A. Rust для программиста на C: словарь соответствий

malloc и free соответствуют Box и Vec, размеченный union соответствует enum, #define соответствуют const и macro_rules, errno соответствует Result.

Ниже этот словарь развёрнут полностью. Каждая строка таблиц отвечает на
вопрос «чем заменить привычную конструкцию C» и ссылается на главу, где
механизм разобран подробно. Там, где перенос «слово в слово» меняет
поведение программы, есть отдельная врезка. Все утверждения о поведении
проверены тестами [крейта][g-crate] `code/appendix-a`, а C-код рядом с ними собран
и запущен.

## Код приложения

```console
$ cargo test -p appendix-a                 # тесты в debug
$ cargo test -p appendix-a --release       # те же тесты в release: часть поведения зависит от профиля
$ cargo +nightly miri test -p appendix-a   # то же под Miri
$ bash code/appendix-a/c/run.sh            # C-примеры под UBSan
ok    uint16_t * uint16_t переполняет int
ok    INT_MIN / -1 под UBSan
...
```

C-примеры лежат в `code/appendix-a/c/`. Скрипт собирает их gcc с
`-fsanitize=undefined` и сверяет вывод с тем, что написано ниже. Вывод
получен на rustc 1.97.0, gcc 13.3 для x86_64 и arm-none-eabi-gcc 13.2.

## A.1. Типы данных

| C | Rust | Замечание |
|---|---|---|
| `int8_t` … `int64_t`, `uint8_t` … `uint64_t` | `i8` … `i64`, `u8` … `u64` | ширина в имени типа; `i128` и `u128` есть на всех [целях][g-target] |
| `int`, `long`, `unsigned` | `c_int`, `c_long`, `c_uint` из `core::ffi` | только на границе с C, в остальном коде ширину выбирают явно |
| `size_t`, `uintptr_t` | `usize` | индексы и длины всегда `usize` ([гл. 1](ch01/01-integers.md)) |
| `ptrdiff_t`, `intptr_t` | `isize` | |
| `char` как байт | `u8`, на границе с C `c_char` | `char` в Rust — [скалярное значение Unicode][g-unicode-scalar], 4 байта |
| `bool`, `_Bool` | `bool` | 1 байт; значение, отличное от 0 и 1, — UB ([гл. 16](ch16/02-invariants.md)) |
| `float`, `double` | `f32`, `f64` | |
| `void` в типе возврата | `()` | [тип нулевого размера][g-zst] |
| `void *` | `*mut c_void`, `*mut u8` или [обобщённый][g-generic] `T` | обобщения вместо стирания типа ([гл. 10](ch10/index.md)) |
| `T *`, не NULL, на один объект | `&T`, `&mut T` | `&mut T` означает единственный доступ ([гл. 4](ch04/index.md)) |
| `T *`, может быть NULL | `Option<&T>`, `Option<NonNull<T>>` | размер тот же, NULL кодирует `None` ([6.4](ch06/04-niches.md)) |
| `T *p, size_t n` | `&[T]`, `&mut [T]` | указатель и длина в одном значении, индекс проверяется |
| `T a[N]` | `[T; N]` | передаётся по значению и в указатель не распадается |
| `const char *` как строка | `&str`, `&CStr`, `&[u8]` | раздел A.6 |
| `struct` | `struct`, для общей с C [раскладки][g-layout] `#[repr(C)]` | [6.1](ch06/01-structs.md), [6.2](ch06/02-repr-c.md) |
| `union` плюс поле-тег | `enum` с данными | [6.3](ch06/03-enums.md) |
| `union` | `union` | поле читается только в `unsafe` ([6.6](ch06/06-unions.md)) |
| `enum` | `enum` без данных, на границе с C — целое | ловушка ниже |
| битовые поля | сдвиги и маски, типы-обёртки регистров | [6.9](ch06/09-registers.md) |
| `typedef` | `type` (синоним) или [newtype][g-newtype] (новый тип) | [6.1](ch06/01-structs.md) |
| `T data[]` в конце структуры | `struct { len: usize, data: [T] }` | [тип динамического размера][g-dst] ([гл. 17](ch17/06-dst.md)) |
| `const`-объект | `const` (подставляется) или `static` (один адрес) | [гл. 25](ch25/01-consts.md) |
| `volatile` | `read_volatile`, `write_volatile` | volatile — свойство доступа, а не типа ([гл. 17](ch17/05-volatile.md)) |
| `restrict` | `&mut T` | компилятор знает это без пометок |
| `_Atomic` | `AtomicU32` и другие | [гл. 21](ch21/01-atomics.md) |
| `_Thread_local` | `thread_local!` | [гл. 20](ch20/index.md) |

Размеры из таблицы проверяет тест `types::tests::sizes`. Отдельно о
массивах: в C имя массива в аргументе функции превращается в указатель, и
функция меняет массив вызывающего. В Rust массив — обычное значение:

```rust
{{#include ../code/appendix-a/src/types.rs:array_by_value}}
```

После `zero_first(a)` массив `a` остаётся `[1, 2, 3, 4]`. Чтобы функция
меняла чужой массив, она должна принять `&mut [u8; 4]` или [срез][g-slice]
`&mut [u8]`, и это видно в её сигнатуре.

<div class="box trap">

**Ловушка: `enum` из C.** Переменная типа `enum mode` в C может хранить
любое значение своего целого типа: библиотека новой версии добавит
`MODE_TURBO = 3`, а ваш код о нём не знает. Для Rust-`enum` значение вне
списка вариантов — неопределённое поведение уже в момент создания, ещё до
первого `match`. Поэтому на границе [FFI][g-ffi] принимают число и проверяют его:

```rust
{{#include ../code/appendix-a/src/types.rs:c_enum}}
```

Прямое преобразование числа в `Mode` Miri ловит сразу:

```console
$ cargo +nightly miri test -p appendix-a -- --ignored ub_enum
error: Undefined Behavior: constructing invalid value of type types::Mode: at .<enum-tag>, encountered 0x07, but expected a valid enum tag
   --> code/appendix-a/src/types.rs:115:35
    |
115 |         let mode: Mode = unsafe { core::mem::transmute(raw) };
    |                                   ^^^^^^^^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
```

</div>

<div class="box ref">

**Справка: C-типы на x86_64 и на ARM.** Ширину `int`, `long`, `char` и
`enum` задаёт [ABI][g-abi] платформы, а не стандарт C.

| | x86_64 Linux (System V) | Cortex-M (`thumb*-none-eabi`, [AAPCS][g-aapcs]) |
|---|---|---|
| `long`, указатель | 8 байт ([LP64][g-lp64]) | 4 байта (ILP32) |
| знак `char` | знаковый | беззнаковый |
| `enum { A, B, C }` | 4 байта | 1 байт |
| `enum { A = 0, B = 300 }` | 4 байта | 2 байта |

Последние две строки — «короткие перечисления» ARM [EABI][g-eabi]: размер
перечисления равен наименьшему целому, в которое помещаются его значения.
arm-none-eabi-gcc 13.2 делает так по умолчанию, и `#[repr(C)]` в Rust
следует тому же ABI. Проверки при компиляции фиксируют это для обеих
архитектур:

```rust
{{#include ../code/appendix-a/src/types.rs:c_sizes}}
```

Ветку для ARM проверяет сборка `cargo build -p appendix-a
--no-default-features --target thumbv7m-none-eabi`. Отсюда два вывода.
`RawMode(u32)` из примера выше годится для Linux, а для C-библиотеки под
Cortex-M сырой тип был бы `u8`; [bindgen][g-bindgen], запущенный для нужной цели, сам
выберет ширину. И `c_char` нельзя заменять на `i8`: на ARM это `u8`.

</div>

## A.2. Целые числа и арифметика

В C у целых чисел три ловушки сразу: знаковое переполнение — UB,
операнды неявно расширяются до `int`, и смешение знаковых с беззнаковыми
молча меняет смысл сравнения. В Rust неявных преобразований между
числовыми типами нет вообще, а переполнение определено.

| Ситуация | C | Rust |
|---|---|---|
| переполнение знакового | UB | [паника][g-panic] в debug, перенос по модулю 2ⁿ в release |
| переполнение беззнакового | перенос по модулю | так же, как у знакового |
| деление на ноль | UB | паника всегда |
| `INT_MIN / -1` | UB, на x86 сигнал SIGFPE | паника всегда, и в release |
| сдвиг на ширину типа и больше | UB | паника в debug, в release число бит по модулю ширины |
| сдвиг вправо отрицательного | до C23 определяется реализацией | арифметический |
| `uint8_t + uint8_t` | расширение до `int` | `u8 + u8` даёт `u8` |
| `int` и `unsigned` в одном выражении | неявное преобразование | ошибка компиляции |
| сужение `(uint8_t)x` | отбрасывает старшие биты | `as` отбрасывает, `try_from` проверяет |
| `double` в `int` вне диапазона | UB | `as` [насыщает][g-saturating], NaN даёт 0 |
| `%` с отрицательными | знак делимого (C99) | так же; `rem_euclid` для остатка ≥ 0 |

Классическая ловушка с расширением выглядит безобидно:

```c
{{#include ../code/appendix-a/c/promote.c:promote}}
```

`uint16_t` меньше `int`, поэтому оба множителя сначала становятся `int`, и
65535 · 65535 = 4 294 836 225 не помещается в `int`:

```text
promote.c:8:14: runtime error: signed integer overflow: 65535 * 65535 cannot be represented in type 'int'
```

Без [UBSan][g-ubsan] программа печатает «правильный» ответ, но это случайность:
компилятор вправе считать, что переполнения не бывает. В Rust
расширения нет, и тип результата — тип операндов:

```rust
{{#include ../code/appendix-a/src/ints.rs:square}}
```

`square_naive(65535)` в debug паникует с сообщением `attempt to multiply
with overflow`, а в release возвращает 1: 0xFFFF² = 0xFFFE_0001, и от
него остаются младшие 16 бит. Оба исхода предсказуемы и проверяются тестом
`promotion_must_be_explicit`. Правильный перенос расширяет операнды явно,
как `square`.

Когда переполнение — часть алгоритма (контрольные суммы, хеши, счётчики
времени), намерение записывают в имени операции:

```rust
{{#include ../code/appendix-a/src/ints.rs:overflow_api}}
```

Есть и тип `core::num::Wrapping<T>`, у которого `+` всегда переносит.

### Деление

Проверки переполнения при `+`, `-` и `*` в release выключены. Деление —
исключение: `i32::MIN / -1` и деление на ноль паникуют в любом [профиле][g-profile].

```rust
{{#include ../code/appendix-a/src/ints.rs:quot}}
```

```c
{{#include ../code/appendix-a/c/divmin.c:divmin}}
```

C-версия под UBSan сообщает `division of -2147483648 by -1 cannot be
represented in type 'int'`, а без проверок на x86_64 процесс получает
SIGFPE и завершается с кодом 136.

<div class="box hood">

**Под капотом.** `appa_quot` из `asm_demo.rs` — та же функция `a / b`.

```text
x86_64:                                thumbv7m:
appa_quot:                             appa_quot:
	push rax                           	push {r7, lr}
	test esi, esi                      	mov r7, sp
	je .LBB40_3                        	cbz r1, .LBB13_3
	mov eax, esi                       	cmp.w r0, #-2147483648
	not eax                            	it eq
	lea ecx, [rdi - 2147483648]        	addseq.w r2, r1, #1
	or ecx, eax                        	beq .LBB13_4
	je .LBB40_2                        	sdiv r0, r0, r1
	mov eax, edi                       	pop {r7, pc}
	cdq                                .LBB13_3:
	idiv esi                           	...
	pop rcx                            	bl core::panicking::panic_const::panic_const_div_by_zero
	ret                                .LBB13_4:
.LBB40_3:                              	...
	...                                	bl core::panicking::panic_const::panic_const_div_overflow
	call ...panic_const_div_by_zero
.LBB40_2:
	...
	call ...panic_const_div_overflow
```

На x86_64 обе проверки нужны самому процессору: `idiv` при делении на ноль
и при `INT_MIN / -1` генерирует исключение #DE, которое Linux превращает в
SIGFPE. Проверка на `-1` сделана без ветвлений: `not eax` равен нулю
только при `b == -1`, `lea` даёт ноль только при `a == INT_MIN`, и `or`
объединяет оба условия.

На Cortex-M3 инструкция `sdiv` не ловушка: деление на ноль даёт 0 (если
не включён бит `CCR.DIV_0_TRP`), а `0x80000000 / -1` даёт `0x80000000`
([ARMv7-M][g-armv7m] Architecture Reference Manual, описание SDIV; в QEMU на машине
`lm3s6965evb` получается ровно это). Rust всё равно вставляет обе проверки,
поэтому программа ведёт себя одинаково на обеих архитектурах. На Cortex-M0
инструкции деления нет вовсе: после тех же проверок вызывается
`__aeabi_idiv` из `compiler_builtins`.

</div>

### Приведения `as`

```rust
{{#include ../code/appendix-a/src/ints.rs:casts}}
```

`as` между целыми никогда не паникует и ничего не проверяет: это
побитовое приведение с отбрасыванием или расширением. Поэтому в новом
коде для сужения берут `try_from`, а `as` оставляют для мест, где
отбрасывание старших бит и есть цель. Приведение вещественного к целому,
наоборот, определено для любого значения: Rust насыщает результат, а в C
`(int)3e9` — UB (gcc 13 без проверок на x86_64 даёт `-2147483648`,
результат инструкции `cvttsd2si` для «неизвестного» значения).

<div class="box trick">

**Трюк.** Если нужна именно семантика C, чтобы оптимизатор мог считать
переполнение невозможным, есть `unchecked_add`, `unchecked_mul` и
`unchecked_shl`. Это `unsafe fn`: переполнение в них — UB, как в C, и
обязанность доказать его отсутствие лежит на вас. Обратная задача —
проверки переполнения в release, как `-ftrapv`, — решается строкой
`overflow-checks = true` в `[profile.release]`.

</div>

## A.3. Память и указатели

| C | Rust | Замечание |
|---|---|---|
| `malloc` + инициализация | `Box::new(x)` | память выделяется уже с значением ([гл. 14](ch14/01-box.md)) |
| `free(p)` | ничего: `drop` в конце области видимости | явно — `drop(p)` |
| `calloc(n, size)` | `vec![0; n]` | просит у аллокатора обнулённую память, как `calloc` |
| `realloc` | `Vec::reserve`, `Vec::resize` | [гл. 15](ch15/01-vec.md) |
| `alloca`, VLA | нет | массивы фиксированного размера или куча |
| `memcpy(d, s, n)` | `d.copy_from_slice(s)` | длины должны совпадать, иначе паника |
| `memmove` | `buf.copy_within(from.., to)` | области могут перекрываться |
| `memset(p, 0, n)` | `buf.fill(0)` | |
| `memcmp` | `a == b`, `a.cmp(b)` для срезов | |
| `sizeof(T)`, `sizeof x` | `size_of::<T>()`, `size_of_val(&x)` | [`const fn`][g-const-fn], работают в константах |
| `_Alignof(T)` | `align_of::<T>()` | |
| `offsetof(T, f)` | `offset_of!(T, f)` | [6.2](ch06/02-repr-c.md) |
| `p + i`, `p[i]` | `s[i]`, `s.get(i)`; для [сырых указателей][g-raw-pointer] `p.add(i)` | `add` — `unsafe` ([гл. 17](ch17/02-ptr.md)) |
| `p - q` | `p.offset_from(q)` | `unsafe` |
| `(uintptr_t)p`, `(T *)addr` | `p.addr()`, `p.with_addr(a)`, `expose_provenance` | [provenance][g-provenance] ([гл. 16](ch16/04-provenance.md)) |
| `*(uint32_t *)buf` | `u32::from_le_bytes(...)`, `read_unaligned` | [6.7](ch06/07-packed.md), [гл. 17](ch17/02-ptr.md) |
| `container_of` | `ptr.byte_sub(offset_of!(T, f))` | `unsafe`, пример ниже |

Функции `mem*` превращаются в методы срезов, которые знают длины своих
аргументов:

```rust
{{#include ../code/appendix-a/src/memory.rs:mem_fns}}
```

`copy_header(&mut hdr, &[1, 2, 3])` не прочитает чужую память: срез
`src[..4]` паникует с сообщением `range end index 4 out of range for slice
of length 3`. Если бы длины разошлись у самого `copy_from_slice`, сообщение
было бы `source slice length (3) does not match destination slice length
(4)`.

`vec![0u8; n]` вызывает у аллокатора `alloc_zeroed`, а системный аллокатор
реализует его через `calloc`. Это наблюдение о текущей стандартной
библиотеке: тест `tests/zeroed.rs` подменяет [глобальный аллокатор][g-global-allocator] на
счётчик и проверяет, что `vec![0; n]` идёт через `alloc_zeroed`, а
`vec![1; n]` нет.

Интрузивные списки ядра Linux держатся на `container_of`: от адреса поля
к адресу объекта. В Rust это та же арифметика, но с одним условием на
происхождение указателя:

```rust
{{#include ../code/appendix-a/src/memory.rs:container_of}}
```

<div class="box trap">

**Ловушка.** Указатель на поле, взятый прямо от переменной, выглядит так
же, но Miri в модели по умолчанию ([Stacked Borrows][g-stacked-borrows]) его отвергает:

```rust
{{#include ../code/appendix-a/src/memory.rs:ub_container_of}}
```

```console
$ cargo +nightly miri test -p appendix-a -- --ignored ub_container_of
error: Undefined Behavior: trying to retag from <178040> for SharedReadOnly permission at alloc54681[0x0], but that tag does not exist in the borrow stack for this location
   --> code/appendix-a/src/memory.rs:116:29
help: <178040> was created by a SharedReadWrite retag at offsets [0x8..0x10]
   --> code/appendix-a/src/memory.rs:115:20
```

Права указателя `node` покрывают байты 8..16, то есть одно поле, а `id`
лежит по смещению 0. Miri сам оговаривается, что правила Stacked Borrows
пока экспериментальные; почему модели [алиасинга][g-aliasing] устроены именно так и чем
они различаются, разбирается в [главе 16](ch16/05-models.md). Безопасный путь
работает в любой из них: указатель на поле получают из указателя на весь
объект, как в `node_of`.

</div>

## A.4. Управление потоком

| C | Rust | Замечание |
|---|---|---|
| `switch` | `match` | проверка [полноты][g-exhaustiveness], без провала в следующую ветку, `\|` для нескольких значений ([гл. 2](ch02/03-match.md)) |
| `cond ? a : b` | `if cond { a } else { b }` | `if` — выражение |
| `for (i = 0; i < n; i++)` | `for i in 0..n` | чаще [итератор][g-iterator] по самой коллекции ([гл. 13](ch13/index.md)) |
| `for (;;)`, `while (1)` | `loop` | `break значение` возвращает результат |
| `do { } while (c)` | `loop { ...; if !c { break } }` | пример ниже |
| `goto` на выход из вложенных циклов | метки `'outer:` и `break 'outer` | |
| `goto err_N` для отката | `?` и `Drop` | пример ниже |
| оператор «запятая» | блок `{ a; b }` | |
| `setjmp` / `longjmp` | нет; паника и `catch_unwind` — не для управления потоком | [гл. 7](ch07/index.md) |
| `exit(code)` | `std::process::exit(code)` | [деструкторы][g-destructor] не вызываются, ловушка ниже |
| `abort()` | `std::process::abort()` | |
| `__builtin_unreachable()` | `unreachable!()` или `unsafe { unreachable_unchecked() }` | первое паникует, второе — UB, если достижимо |

Драйвер на C, который захватывает несколько ресурсов, обычно выглядит так:

```c
int dev_init(struct dev *d)
{
    if (clk_get(&d->clk))
        return -EBUSY;
    if (dma_get(&d->dma))
        goto err_clk;
    if (irq_get(&d->irq))
        goto err_dma;
    return 0;
err_dma:
    dma_put(&d->dma);
err_clk:
    clk_put(&d->clk);
    return -EBUSY;
}
```

Порядок меток надо поддерживать вручную, и ошибка в нём — утечка или
двойное освобождение. В Rust освобождение привязано к [владению][g-ownership]:

```rust
{{#include ../code/appendix-a/src/control.rs:cleanup}}
```

Если `irq` занят, журнал теста `cleanup_in_reverse_order` получается
`["get clk", "get dma", "free dma", "free clk"]`: локальные переменные
уничтожаются в порядке, обратном созданию. Готовое устройство при
уничтожении освобождает поля в порядке объявления, и это тоже проверено.

Выход из вложенных циклов и цикл с проверкой в конце:

```rust
{{#include ../code/appendix-a/src/control.rs:labeled}}
```

```rust
{{#include ../code/appendix-a/src/control.rs:do_while}}
```

<div class="box trap">

**Ловушка: `exit` в C и `process::exit` в Rust делают разное.** В C
`exit` вызывает обработчики `atexit`, и libc сбрасывает буферы stdio:

```c
{{#include ../code/appendix-a/c/exit_flush.c:exit}}
```

Строка будет напечатана, даже если вывод идёт в канал, где stdio
буферизует полностью. Rust-версия теряет данные:

```rust
{{#include ../code/appendix-a/src/bin/exit_skips_drop.rs:exit}}
```

`process::exit` завершает процесс сразу, деструкторы объектов на стеке не
выполняются, и `BufWriter` не успевает вызвать `flush`. Тест `tests/exit.rs`
запускает эту программу и проверяет, что её stdout пуст. Правило:
возвращайтесь из `main`, а код завершения передавайте через
`std::process::ExitCode`.

</div>

## A.5. Ошибки

| C | Rust | Замечание |
|---|---|---|
| `return -1` и `errno` | `Result<T, E>` | ошибка — значение, её нельзя не заметить ([гл. 7](ch07/index.md)) |
| `NULL` как «не найдено» | `Option<T>` | |
| `if (rc < 0) return rc;` | `?` | с преобразованием через `From` |
| `perror(msg)` | `eprintln!("{msg}: {e}")` | `io::Error` печатает текст `strerror` |
| `errno` после вызова libc | `io::Error::last_os_error()` | читать сразу после вызова |
| `__attribute__((warn_unused_result))` | `#[must_use]` | `Result` помечен им в стандартной библиотеке |
| `assert(c)` | `assert!(c)`, `debug_assert!(c)` | `assert!` не исчезает в release, в отличие от `NDEBUG` |
| перечисление кодов ошибок | `enum Error` + `impl core::error::Error` | |

```rust
{{#include ../code/appendix-a/src/errors.rs:errno}}
```

Для несуществующего файла тест получает `ErrorKind::NotFound`,
`raw_os_error() == Some(2)` (ENOENT) и текст
`No such file or directory (os error 2)`. Если функцию libc вызывают
напрямую, errno читают так:

```rust
{{#include ../code/appendix-a/src/errors.rs:last_os_error}}
```

Обратите внимание на комментарий к `close`: в C закрыть «чужой»
дескриптор — обычная ошибка, а в Rust безопасная функция, которая это
позволяет, была бы [unsound][g-soundness]. Чек-лист таких рассуждений — в
[приложении C](appendix-c.md).

## A.6. Строки и форматированный вывод

| C | Rust | Замечание |
|---|---|---|
| `char *` с NUL в конце | `&str` | UTF-8, длина хранится, NUL нет |
| строка для функции C | `&CStr`, `CString` | [гл. 18](ch18/03-strings.md) |
| произвольные байты | `&[u8]`, `Vec<u8>` | |
| `"abc"` | `"abc"`: `&'static str`; `c"abc"`: `&'static CStr`; `b"abc"`: `&[u8; 3]` | |
| `strlen(s)` | `s.len()` | за O(1); для `&CStr` — `count_bytes()`, считает до NUL |
| `strcmp` | `==`, `cmp` | |
| `strcpy`, `strncpy`, `strlcpy` | `to_owned`, `push_str`, `copy_from_slice` | |
| `strstr`, `strchr`, `strtok` | `find`, `split`, `split_whitespace` | |
| `sprintf`, `snprintf` | `format!`, `write!` | `write!` пишет в любой `fmt::Write` |
| `printf` | `print!`, `println!` | формат проверяется при компиляции |
| `atoi`, `strtol` | `s.parse::<i32>()`, `i64::from_str_radix(s, 16)` | ошибка отличима от нуля |
| `s[i]` | `s.as_bytes()[i]` | `str` по номеру байта не индексируется |
| `toupper`, `isdigit` | `to_ascii_uppercase`, `is_ascii_digit` | |

Форматы `printf` переводятся почти механически. Тест сравнивает вывод
`format!` с тем, что печатает `c/printf.c`:

```rust
{{#include ../code/appendix-a/src/strings.rs:printf_map}}
```

| `printf` | `format!` |
|---|---|
| `%d`, `%u`, `%zu`, `%s`, `%c` | `{}` |
| `%5d`, `%-5d`, `%05d`, `%+d` | `{:5}`, `{:<5}`, `{:05}`, `{:+}` |
| `%x`, `%X`, `%#x`, `%08x`, `%#010x`, `%o` | `{:x}`, `{:X}`, `{:#x}`, `{:08x}`, `{:#010x}`, `{:o}` |
| `%.3f`, `%8.2f` | `{:.3}`, `{:8.2}` |
| `%10s`, `%-10s`, `%.3s` | `{:>10}`, `{:<10}`, `{:.3}` |
| `%p` | `{:p}` |
| `%%` | `%`; фигурные скобки удваиваются: `{{`, `}}` |
| нет | `{:b}`, `{:?}`, `{:#?}` |

<div class="box trap">

**Ловушка.** В `printf` ширина без флага выравнивает вправо всё подряд. В
Rust числа выравниваются вправо, а строки влево: `format!("[{:10}]",
"abcdef")` даёт `[abcdef    ]`, тогда как `printf("[%10s]", "abcdef")`
даёт `[    abcdef]`. Для строк пишите направление явно: `{:>10}`. Второе
отличие — `{:e}`: мантисса кратчайшая, а экспонента без знака и
ведущих нулей, `1.23456e0` вместо `1.234560e+00`.

</div>

`snprintf` в буфер на стеке без кучи — частая нужда в прошивках. Хватает
своего типа с `fmt::Write`:

```rust
{{#include ../code/appendix-a/src/strings.rs:fixed_buf}}
```

В отличие от `snprintf`, который молча обрезает строку, `write!` вернёт
ошибку. Уже записанные куски при этом остаются: аргументы форматируются
по очереди, и в тесте `snprintf_like` из `", {n}"` в буфер попадает
только `", "`.

Разбор чисел возвращает ошибку, которую нельзя спутать с нулём:

```rust
{{#include ../code/appendix-a/src/strings.rs:parse}}
```

И последнее: `str` хранит UTF-8, поэтому длина в байтах и число символов
различаются, а срез посреди символа паникует:

```rust
{{#include ../code/appendix-a/src/strings.rs:utf8}}
```

`&s[0..1]` для строки «привет» завершается паникой `byte index 1 is not a
char boundary`.

## A.7. Функции и обратные вызовы

| C | Rust | Замечание |
|---|---|---|
| `int (*f)(int)` | `fn(i32) -> i32` | никогда не NULL; `Option<fn(...)>` того же размера |
| колбэк + `void *ctx` | [замыкание][g-closure]: `impl FnMut(...)`, `&mut dyn FnMut(...)` | контекст типизирован ([гл. 12](ch12/index.md)) |
| `static` функция в файле | функция без `pub` | видимость на уровне [модуля][g-module] ([гл. 8](ch08/01-modules.md)) |
| `extern` объявление | `use`; для C — `unsafe extern "C" { ... }` | [гл. 18](ch18/01-abi.md), [приложение B](appendix-b.md) |
| `inline`, `__attribute__((always_inline))`, `noinline` | `#[inline]`, `#[inline(always)]`, `#[inline(never)]` | |
| `__attribute__((noreturn))` | `-> !` | |
| `...` (переменное число аргументов) | макрос; вызов C-функции с `...` возможен | [гл. 18](ch18/index.md), [23.2](ch23/02-repeat.md) |
| перегрузка, аргументы по умолчанию | нет; [трейты][g-trait], `Option`, строители | |
| `static` переменная в функции | `static` с [атомиком][g-atomic], `OnceLock`, `LazyLock` | [гл. 14](ch14/06-lazy.md), [гл. 25](ch25/04-statics.md) |
| `__attribute__((constructor))` | нет: до `main` код не выполняется | явная инициализация или `LazyLock` |
| `qsort(v, n, size, cmp)` | `v.sort_unstable_by(\|a, b\| ...)` | [мономорфизация][g-monomorphization], компаратор [встраивается][g-inlining] ([гл. 10](ch10/06-inlining.md)) |

```rust
{{#include ../code/appendix-a/src/funcs.rs:callback}}
```

Замыкание само хранит захваченные переменные, поэтому нет ни `void *`, ни
приведения типа в колбэке. Сколько оно занимает, видно по тестам:

```rust
{{#include ../code/appendix-a/src/funcs.rs:fn_sizes}}
```

<div class="box c">

**Как в C.** Компаратор для `qsort` через вычитание встречается и в
учебниках:

```c
{{#include ../code/appendix-a/c/qsort.c:cmp}}
```

Для массива `{INT_MAX, -1, INT_MIN, 0}` UBSan сообщает `signed integer
overflow: 2147483647 - -1 cannot be represented in type 'int'`, а без
проверок `qsort` выдаёт `2147483647 -2147483648 -1 0`, то есть массив не
отсортирован. В Rust компаратор возвращает `Ordering`, а не разность:

```rust
{{#include ../code/appendix-a/src/funcs.rs:sort}}
```

</div>

## A.8. Препроцессор и атрибуты компилятора

| C | Rust | Замечание |
|---|---|---|
| `#include` | `mod`, `use` | заголовков нет ([гл. 8](ch08/01-modules.md)) |
| `#define N 16` | `const N: usize = 16;` | типизирована, видна отладчику |
| `#define MAX(a, b) ...` | `fn`, `core::cmp::max`, [`macro_rules!`][g-declarative-macro] | ловушка ниже |
| `#ifdef DEBUG` | `#[cfg(debug_assertions)]`, `cfg!(...)` | |
| `#if defined(__x86_64__)` | `#[cfg(target_arch = "x86_64")]` | также `target_os`, `target_has_atomic` |
| `-DFEATURE` | [возможности Cargo][g-feature] (`features`), `--cfg` | [гл. 8](ch08/04-features.md) |
| `#error` | `compile_error!` | |
| `_Static_assert` | `const _: () = assert!(...);` | [6.2](ch06/02-repr-c.md) |
| `__FILE__`, `__LINE__` | `file!()`, `line!()` | `#[track_caller]` и `Location::caller()` для места вызова |
| `__func__` | нет; `module_path!()` даёт путь модуля | |
| `#pragma pack(1)`, `__attribute__((packed))` | `#[repr(packed)]` | [6.7](ch06/07-packed.md) |
| `__attribute__((aligned(64)))` | `#[repr(align(64))]` | [6.8](ch06/08-align.md) |
| `__attribute__((section(".x")))` | `#[unsafe(link_section = ".x")]` | [приложение B](appendix-b.md) |
| `__attribute__((used))` | `#[used]` | |
| `__attribute__((weak))` | нет в stable | |
| `__attribute__((cold))` | `#[cold]` | |
| `__attribute__((deprecated))` | `#[deprecated]` | |
| `__builtin_clz`, `__builtin_ctz` | `leading_zeros`, `trailing_zeros` | определены и для нуля |
| `__builtin_popcount` | `count_ones` | |
| `__builtin_bswap32`, `htonl`, `ntohl` | `swap_bytes`, `to_be`, `u32::from_be_bytes` | |
| `__builtin_add_overflow` | `checked_add`, `overflowing_add` | |
| `__builtin_prefetch` | `core::arch::x86_64::_mm_prefetch` | |
| `asm volatile("" ::: "memory")` | `compiler_fence(Ordering::SeqCst)` | смысл не совпадает полностью ([гл. 21](ch21/03-fences.md)) |
| `__sync_synchronize()` | `fence(Ordering::SeqCst)` | |

<div class="box trap">

**Ловушка: макрос вычисляет аргумент столько раз, сколько он подставлен.**

```c
{{#include ../code/appendix-a/c/max.c:max}}
```

Программа печатает `m = 6, i = 7`: `i++` выполнился дважды. [Гигиена][g-hygiene]
`macro_rules!` защищает имена переменных, но не число вычислений, и
буквальный перенос ведёт себя точно так же:

```rust
{{#include ../code/appendix-a/src/preproc.rs:max_macro}}
```

Тест `macro_double_evaluation` повторяет C-пример с замыканием вместо
`i++`: `max_twice!` даёт `(m, i) = (6, 7)`, `max_once!` — `(5, 6)`.
Привязка аргументов через `match` (или `let`) вычисляет каждое выражение
ровно один раз. Но если макрос не нужен для синтаксиса, лучше функция.

</div>

Встроенные функции GCC стали методами целых типов, и у них нет
неопределённых входов:

```rust
{{#include ../code/appendix-a/src/preproc.rs:builtins}}
```

<div class="box hood">

**Под капотом.** `leading_zeros` определён и для нуля, а `__builtin_clz(0)`
нет (gcc с UBSan: `passing zero to clz(), which is not a valid argument`).
Во что обходится определённость, видно по `appa_clz`:

```text
x86_64:                     x86_64, -C target-cpu=haswell:
appa_clz:                   appa_clz:
	mov eax, 63             	lzcnt eax, edi
	bsr eax, edi            	ret
	xor eax, 31
	ret

thumbv7m:                   thumbv6m:
appa_clz:                   appa_clz:
	push {r7, lr}           	push {r7, lr}
	mov r7, sp              	add r7, sp, #0
	clz r0, r0              	cmp r0, #0
	pop {r7, pc}            	beq .LBB12_2
	                        	bl __clzsi2
	                        	pop {r7, pc}
	                        .LBB12_2:
	                        	movs r0, #32
	                        	pop {r7, pc}
```

`bsr` возвращает номер старшего установленного бита, и для нуля ответа
у него нет: инструкция только поднимает флаг ZF, а приёмник оставляет
прежним (AMD это документирует, Intel формально нет, LLVM на это
полагается). Поэтому компилятор заранее кладёт в `eax` число 63, и для
нуля получается `63 xor 31 = 32`. Процессоры с `lzcnt` (Haswell и новее)
обрабатывают ноль сами. Инструкция `clz` в ARMv7-M для нуля возвращает 32,
поэтому на Cortex-M3 определённость ничего не стоит. В ARMv6-M `clz` нет,
и ноль проверяется отдельно перед вызовом `__clzsi2`.

</div>

## A.9. Потоки и синхронизация

| C (POSIX) | Rust | Замечание |
|---|---|---|
| `pthread_create`, `pthread_join` | `thread::spawn`, `JoinHandle::join` | [`thread::scope`][g-scoped-threads] для [заимствования][g-borrowing] данных со стека ([гл. 20](ch20/04-scoped.md)) |
| `pthread_mutex_t` рядом с данными | `Mutex<T>` с данными внутри | доступ только через [guard][g-guard] |
| `pthread_rwlock_t` | `RwLock<T>` | |
| `pthread_cond_t` | `Condvar` | |
| `pthread_once` | `Once`, `OnceLock`, `LazyLock` | |
| `_Atomic`, `atomic_fetch_add_explicit` | `AtomicU64::fetch_add(v, Ordering)` | [порядок памяти][g-memory-ordering] обязателен ([гл. 21](ch21/02-ordering.md)) |
| `volatile int flag` между потоками | `AtomicBool` | `volatile` не синхронизирует ни в C, ни в Rust |
| `__thread`, `_Thread_local` | `thread_local!` | |
| гонка данных | ошибка компиляции | [`Send`][g-send-sync] и `Sync` ([гл. 20](ch20/02-marker.md)) |
| `signal`, `sigaction` | нет в std; `libc::sigaction`, крейт `signal-hook` | [28.4](ch28/04-signals.md) |
| `fork` | `std::process::Command`; сам `fork` — через libc и `unsafe` | [28.3](ch28/03-processes.md) |

```rust
{{#include ../code/appendix-a/src/threads.rs:mutex}}
```

Забыть взять мьютекс нельзя: данных вне `Mutex` нет. Забыть отпустить
тоже нельзя: guard отпускает его в деструкторе, в том числе при панике.
Глобальный счётчик — атомик, а не `static mut`:

```rust
{{#include ../code/appendix-a/src/threads.rs:atomic}}
```

Гонку данных компилятор не пропустит:

```rust,ignore
{{#include ../code/appendix-a/src/threads.rs:rc_race}}
```

У `Rc` неатомарный счётчик ссылок, поэтому `Rc` не реализует `Send`.
Ошибку E0277 проверяет [doc-тест][g-doctest] в `threads.rs`.

## A.10. Сборка и инструменты

| C | Rust | Замечание |
|---|---|---|
| `make`, CMake | `cargo build` | [гл. 0](ch00/02-cargo.md), [гл. 8](ch08/02-crates.md) |
| `-O0`, `-O2`, `-Os`, `-Oz` | `opt-level = 0`, `2`, `"s"`, `"z"` в `[profile]` | release по умолчанию `opt-level = 3` |
| `-g` | `debug = true` | |
| `-flto` | `lto = true` или `"thin"` | |
| `-ffunction-sections -Wl,--gc-sections` | по умолчанию | |
| `-march=native` | `-C target-cpu=native` | |
| `-mcpu=cortex-m4 -mthumb -mfloat-abi=hard` | `--target thumbv7em-none-eabihf` | |
| `-ffreestanding -nostdlib` | [`#![no_std]`][g-no-std], `#![no_main]` | [гл. 26](ch26/index.md) |
| `-fno-exceptions` | `panic = "abort"` | [гл. 7](ch07/index.md) |
| `-ftrapv` | `overflow-checks = true` | |
| `-fwrapv` | `wrapping_*`, `Wrapping<T>` | |
| `-fno-strict-aliasing` | не нужен: алиасинг на основе типов в Rust не используется | правила для `&` и `&mut` ([гл. 4](ch04/index.md)) |
| `-Wall -Wextra -Werror` | `cargo clippy -- -D warnings` | |
| `-fsanitize=address`, `thread` | `-Zsanitizer=address`, `thread` (nightly) | для `unsafe` — ещё и Miri ([гл. 31](ch31/06-miri-loom.md)) |
| [скрипт компоновки][g-linker-script] `-T` | `memory.x` и `-C link-arg=-Tlink.x` | `firmware/.cargo/config.toml` |
| `.a`, `.so` | `crate-type = ["staticlib"]`, `["cdylib"]` | [гл. 8](ch08/06-crate-types.md), [гл. 18](ch18/04-owned.md) |
| `pkg-config`, `-lfoo` | `build.rs` и `cargo:rustc-link-lib=foo` | |
| `objdump -d`, `nm` | `cargo asm`, `llvm-objdump`, `nm` | [гл. 0](ch00/03-asm.md) |

## A.11. Что в C — неопределённое поведение

| Ситуация | C | безопасный Rust | `unsafe` Rust |
|---|---|---|---|
| использование после `free` | UB | ошибка компиляции E0382 | UB |
| двойное освобождение | UB | невозможно: владелец один | UB |
| указатель на локальную переменную после возврата | UB | ошибка компиляции E0515 | UB |
| чтение неинициализированной переменной | UB | ошибка компиляции E0381 | UB (`MaybeUninit`) |
| изменение коллекции во время обхода | UB после `realloc` | ошибка компиляции E0502 | UB |
| выход за границы массива | UB | паника | UB (`get_unchecked`) |
| разыменование NULL | UB | ссылок со значением NULL нет | UB |
| гонка данных | UB | ошибка компиляции (`Send`, `Sync`) | UB |
| знаковое переполнение | UB | определено (раздел A.2) | `unchecked_add`: UB |
| сдвиг на ширину типа | UB | определено | `unchecked_shl`: UB |
| `double` в `int` вне диапазона | UB | насыщение | `to_int_unchecked`: UB |
| чтение `float` через `int *` | UB (strict aliasing) | `f32::to_bits`, `from_bits` | допустимо, если значение валидно |
| невыровненное разыменование | UB | — | UB; `read_unaligned` |
| изменение строкового литерала | UB | `&'static str` неизменяем | UB |
| бесконечный цикл без побочных эффектов | при неконстантном условии компилятор вправе считать цикл завершающимся (C11 6.8.5p6); `for (;;);` и `while (1);` — нет | определён: так пишут `panic_handler` | определён |

Ошибки компиляции из таблицы проверяют doc-тесты в `ub.rs`:

```rust,ignore
{{#include ../code/appendix-a/src/ub.rs:compile_errors}}
```

Столбец «`unsafe` Rust» важен не меньше двух других: внутри `unsafe` те
же ошибки снова становятся неопределённым поведением, и последствия у них
те же, что в C. Разница в том, где их искать. Как проверять такой код,
описано в [приложении C](appendix-c.md).

<div class="box check">

**Проверь себя.**

1. Что вернёт `square_naive(65535)` в debug и в release? Почему тот же
   код на C — неопределённое поведение, хотя все типы в нём беззнаковые?
2. Почему Rust проверяет `i32::MIN / -1` даже в release, хотя проверки
   переполнения для `+` там выключены?
3. C-функция `void set_mode(enum mode m)` из библиотеки для Cortex-M.
   Каким типом объявить параметр в `unsafe extern "C"` и почему не
   Rust-`enum`?
4. Что напечатают `format!("[{:6}]", "ab")` и `printf("[%6s]", "ab")`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-aapcs]: appendix-f.md#aapcs "AAPCS (Procedure Call Standard for the Arm Architecture): Соглашение о вызовах 32-битного ARM: целые аргументы в r0–r3, результат в r0 (и r1), регистры r4–r11 сохраняет вызываемая функция, адрес возврата лежит в lr."
[g-abi]: appendix-f.md#abi "ABI (application binary interface): Двоичный интерфейс платформы: соглашение о вызовах, раскладка типов в памяти, имена символов."
[g-aliasing]: appendix-f.md#aliasing "Алиасинг (aliasing): Два пути к одной памяти: два указателя, ссылка и указатель."
[g-armv7m]: appendix-f.md#armv7m "ARMv6-M, ARMv7-M, ARMv7E-M: Версии архитектуры ARM для микроконтроллеров."
[g-atomic]: appendix-f.md#atomic "Атомик (atomic): Тип из core::sync::atomic (AtomicU32, AtomicBool и другие), операции над которым неделимы и упорядочены по выбранной модели памяти."
[g-bindgen]: appendix-f.md#bindgen "bindgen: Генератор привязок: читает заголовки C через libclang и пишет объявления Rust для их функций, структур и констант."
[g-borrowing]: appendix-f.md#borrowing "Заимствование (borrowing): Создание ссылки на значение без передачи владения: &x — разделяемое, &mut x — уникальное."
[g-closure]: appendix-f.md#closure "Замыкание (closure): Функция вместе со своим контекстом — захваченными переменными."
[g-const-fn]: appendix-f.md#const-fn "const fn: Функция, которую компилятор умеет выполнить при компиляции своим интерпретатором, когда её результат нужен в константном контексте."
[g-crate]: appendix-f.md#crate "Крейт (crate): Единица компиляции Rust: одно дерево модулей, один вызов rustc, один результат — библиотека или программа."
[g-declarative-macro]: appendix-f.md#declarative-macro "Декларативный макрос (declarative macro): Макрос macro_rules!: список правил «образец — подстановка»."
[g-destructor]: appendix-f.md#destructor "Деструктор (destructor): Код, который выполняется при уничтожении значения: реализация трейта Drop, затем уничтожение полей."
[g-doctest]: appendix-f.md#doctest "doc-тест (doctest): Пример кода в документирующем комментарии ///, который cargo test собирает и запускает как тест."
[g-dst]: appendix-f.md#dst "Тип динамического размера (dynamically sized type, DST): Тип, размер которого при компиляции неизвестен: [T], str, dyn Trait, структура с таким последним полем."
[g-eabi]: appendix-f.md#eabi "EABI (Embedded ABI): ABI ARM для встраиваемых систем: соглашение о вызовах AAPCS, раскладка структур, работа со стеком."
[g-exhaustiveness]: appendix-f.md#exhaustiveness "Полнота (exhaustiveness): Требование к match: ветки обязаны покрыть все возможные значения."
[g-feature]: appendix-f.md#feature "Возможность (feature): Именованный переключатель пакета Cargo, который превращается в условие feature для #[cfg]."
[g-ffi]: appendix-f.md#ffi "FFI (foreign function interface): Вызов функций, написанных на другом языке, и показ своих функций ему."
[g-generic]: appendix-f.md#generic "Обобщение (generic): Функция или тип с параметрами-типами: fn max(a: T, b: T) -> T."
[g-global-allocator]: appendix-f.md#global-allocator "Глобальный аллокатор (global allocator): Распределитель памяти, который зовут Box, Vec, String и все коллекции alloc: тип, реализующий GlobalAlloc и объявленный атрибутом #[global_allocator]."
[g-guard]: appendix-f.md#guard "Страж (guard): Значение, чей деструктор отпускает ресурс: MutexGuard отпускает мьютекс, IrqGuard из главы 3 снова разрешает прерывания."
[g-hygiene]: appendix-f.md#hygiene "Гигиена (hygiene): Свойство макросов: имена из тела макроса и имена вызывающего не смешиваются, даже если пишутся одинаково."
[g-inlining]: appendix-f.md#inlining "Встраивание (inlining): Подстановка тела функции в место вызова."
[g-iterator]: appendix-f.md#iterator "Итератор (iterator): Тип, реализующий трейт Iterator: состояние обхода с методом next, который выдаёт следующий элемент или None."
[g-layout]: appendix-f.md#layout "Раскладка (layout): Размер, выравнивание и смещения полей типа в памяти."
[g-linker-script]: appendix-f.md#linker-script "Скрипт компоновщика (linker script): Файл, который говорит компоновщику, по каким адресам разместить секции программы: таблицу векторов — по адресу 0, код и константы — во флеш-память, .data и .bss — в ОЗУ."
[g-lp64]: appendix-f.md#lp64 "LP64, ILP32: Модели данных C."
[g-memory-ordering]: appendix-f.md#memory-ordering "Порядок памяти (memory ordering): Аргумент Ordering атомарной операции: какие из соседних обращений к памяти другие потоки обязаны увидеть до неё или после."
[g-module]: appendix-f.md#module "Модуль (module): Пространство имён внутри крейта со своими правилами видимости: объявляется словом mod, с телом в скобках или в отдельном файле."
[g-monomorphization]: appendix-f.md#monomorphization "Мономорфизация (monomorphization): Создание отдельной копии обобщённой функции для каждого набора типов, с которым её вызвали."
[g-newtype]: appendix-f.md#newtype "Newtype: Кортежная структура с одним полем, которая делает из существующего типа новый: struct Millimeters(i32)."
[g-no-std]: appendix-f.md#no-std "no_std: Атрибут крейта #![no_std]: крейт пользуется только core (и, если нужно, alloc), без стандартной библиотеки ОС."
[g-ownership]: appendix-f.md#ownership "Владение (ownership): Правило Rust: у каждого значения ровно один владелец — переменная, поле или элемент коллекции, — и он отвечает за уничтожение значения."
[g-panic]: appendix-f.md#panic "Паника (panic): Аварийная реакция на ошибку в самой программе: индекс за границей, unwrap на None, переполнение в отладочной сборке."
[g-profile]: appendix-f.md#profile "Профиль (profile): Именованный набор параметров компиляции cargo: уровень оптимизации, отладочная информация, проверки переполнения, стратегия паники."
[g-provenance]: appendix-f.md#provenance "Provenance (происхождение указателя): Невидимая часть указателя: к какому выделению памяти он относится и что через него можно трогать."
[g-raw-pointer]: appendix-f.md#raw-pointer "Сырой указатель (raw pointer): Указатель *const T или *mut T без гарантий ссылки: он может быть нулевым, висячим, невыровненным."
[g-saturating]: appendix-f.md#saturating "Насыщающая арифметика (saturating arithmetic): Арифметика, в которой результат за границей диапазона заменяется ближайшей границей: 200u8.saturating_add(100) равно 255."
[g-scoped-threads]: appendix-f.md#scoped-threads "Область потоков (scoped threads): Блок std::thread::scope, внутри которого порождённые потоки гарантированно присоединяются до выхода из блока."
[g-send-sync]: appendix-f.md#send-sync "Send, Sync: Автотрейты потокобезопасности."
[g-slice]: appendix-f.md#slice "Срез (slice): Непрерывная последовательность элементов, длина которой известна только во время выполнения: [T], а str — срез байт UTF-8."
[g-soundness]: appendix-f.md#soundness "Корректность (soundness): Свойство безопасного API над unsafe: никакой безопасный код, как бы он этот API ни вызывал, не получит UB."
[g-stacked-borrows]: appendix-f.md#stacked-borrows "Stacked Borrows, Tree Borrows: Две модели алиасинга, по которым Miri проверяет ссылки и сырые указатели."
[g-target]: appendix-f.md#target "Цель (target): То, для чего генерируется код: архитектура, ОС и ABI, записанные в имени-триплете (thumbv7m-none-eabi)."
[g-trait]: appendix-f.md#trait "Трейт (trait): Набор методов, типов и констант, которые тип обязуется предоставить: Drop, Iterator, Copy."
[g-ubsan]: appendix-f.md#ubsan "UBSan (UndefinedBehaviorSanitizer): Санитайзер GCC и Clang, который ловит неопределённое поведение C во время выполнения: знаковое переполнение, сдвиг на ширину типа и больше, деление на ноль, невыровненный указатель."
[g-unicode-scalar]: appendix-f.md#unicode-scalar "Скалярное значение Unicode (Unicode scalar value): Число от 0 до 0x10FFFF, кроме суррогатов 0xD800..=0xDFFF."
[g-zst]: appendix-f.md#zst "Тип нулевого размера (zero-sized type, ZST): Тип, значения которого не занимают памяти: (), единичные структуры, PhantomData, [T; 0], перечисление из одного варианта без данных."
