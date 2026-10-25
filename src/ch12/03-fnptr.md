# 12.3. Функции как значения

## Тип функции и указатель на функцию

Имя функции в выражении — не адрес, как в C, а значение особого типа.
У каждой функции свой тип, у типа нет имени в языке (компилятор пишет
его как `fn(u8) -> u8 {double}`), и размер его — ноль: что вызывать,
известно из типа, хранить нечего.

```rust
{{#include ../../code/ch12/src/fnptr.rs:items}}
```

`item_sizes()` возвращает `[0, 8, 8]` на x86-64 и `[0, 4, 4]` на
Cortex-M3. Указатель на функцию `fn(u8) -> u8` — обычный адрес в одно
слово, и `Option` от него не больше слова: указатель на функцию не
бывает нулевым, и `None` занимает ноль.

Раз типы у функций разные, переменной, в которую положили `double`,
нельзя присвоить `invert`, хотя сигнатуры совпадают:

```console
error[E0308]: mismatched types
2 | let mut f = double;
  |             ------ expected due to this value
3 | f = invert;
  |     ^^^^^^ expected fn item, found a different fn item
  = note: expected fn item `fn(_) -> _ {double}`
             found fn item `fn(_) -> _ {invert}`
  = note: different fn items have unique types, even if their signatures are the same
  = help: consider casting both fn items to fn pointers using `as fn(u8) -> u8`
```

Общий тип у них — указатель, и к нему функция приводится сама, если
тип указан явно: `let mut f: fn(u8) -> u8 = double;`. Ровно так же
приводится замыкание без захвата: его структура пуста, и вызвать его
можно, не передавая ничего, кроме аргументов. Замыкание с захватом
указателем не станет — состояние некуда деть:

```console
error[E0308]: mismatched types
3 | let f: fn(u8) -> u8 = |x| x * k;
  |        ------------   ^^^^^^^^^ expected fn pointer, found closure
  |        |
  |        expected due to this
note: closures can only be coerced to `fn` types if they do not capture any variables
3 | let f: fn(u8) -> u8 = |x| x * k;
  |                               ^ `k` captured here
```

Обе ошибки проверяют doc-тесты `code/ch12/src/fnptr.rs`.

## Таблица обработчиков

Указатель на функцию нужен там, где тип нельзя оставить параметром:
в массиве разных обработчиков, в поле структуры, которую видит C, в
[таблице векторов][g-vector-table] прерываний. Функции и замыкания без захвата ложатся в
такой массив вперемешку:

```rust
{{#include ../../code/ch12/src/fnptr.rs:coerce}}
```

Диспетчер — [проверка границы][g-bounds-check] и косвенный переход:

```console
$ cargo asm -p ch12 --lib --simplify ch12_dispatch
ch12_dispatch:
	cmp rdi, 3
	ja .LBB37_1
	mov rax, rdi
	mov rcx, qword ptr [rip + ch12::fnptr::HANDLERS@GOTPCREL]
	movzx edi, sil
	jmp qword ptr [rcx + 8*rax]
.LBB37_1:
	mov eax, esi
	ret
```

```console
$ cargo asm -p ch12 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch12_dispatch
ch12_dispatch:
	push {r7, lr}
	mov r7, sp
	cmp r0, #3
	itt hi
	movhi r0, r1
	pophi {r7, pc}
	movw r2, :lower16:ch12::fnptr::HANDLERS
	movt r2, :upper16:ch12::fnptr::HANDLERS
	ldr.w r2, [r2, r0, lsl #2]
	mov r0, r1
	pop.w {r7, lr}
	bx r2
```

Это тот же код, что дал бы `switch` по [таблице переходов][g-jump-table] в C: обработчик
вызывается хвостовым переходом (`jmp` на x86-64, `bx` на Cortex-M3), и
`ch12_dispatch` не оставляет своего кадра. На Cortex-M0 rustc 1.97
хвостового вызова не делает и зовёт обработчик через `blx`.

Таблица — `static` без [внутренней изменяемости][g-interior-mut], и на Cortex-M она лежит
во flash вместе с кодом. Прошивка `firmware/ch12-closures` печатает её
адрес и адреса четырёх обработчиков (QEMU, `lm3s6965evb`, Cortex-M3):

```console
$ cd firmware && cargo run --release -p ch12-closures
…
HANDLERS: 0x00002b6c во flash
  [0] 0x0000187d -> 0x42
  [1] 0x00001885 -> 0x5e
  [2] 0x0000188d -> 0x50
  [3] 0x00001897 -> 0x1a
адресов с битом Thumb: 4 из 4
```

<div class="box ref">

**Справка: бит Thumb.** Ядра Cortex-M исполняют только набор инструкций
Thumb, а архитектура ARM унаследовала правило: младший бит адреса
перехода в `bx` и `blx` выбирает набор инструкций, 1 — Thumb. Поэтому
все адреса функций нечётные: `0x187d` — это функция по адресу `0x187c`.
Переход по чётному адресу на Cortex-M — исключение [UsageFault][g-faults] (INVSTATE).
Компоновщик ставит этот бит сам, и в таблице векторов прерываний он тоже
стоит. Если адрес функции приходит из C или из ассемблера, бит должен
быть уже установлен; `as usize` в Rust его не снимает, и сравнивать такие
адреса с границами секций нужно с этим учётом.

</div>

На x86-64 та же таблица лежит в секции `.data.rel.ro`, а не в `.rodata`:
исполняемый файл по умолчанию позиционно-независимый, адреса функций
известны только после загрузки, и загрузчик дописывает их в таблицу, а
затем делает страницу только для чтения (RELRO). На Cortex-M адреса
известны при компоновке, и таблице место во flash.

<div class="box hood">

**Под капотом.** Что лежит в элементах `[2]` и `[3]`? Замыкание без
захвата не функция, а структура нулевого размера с методом. При
приведении к указателю rustc создаёт функцию-переходник, которая
вызывает метод на пустой структуре; в таблице символов объектного файла
она называется `<{closure} as FnOnce>::call_once` и занимает 5 и 6 байт
на x86-64 — само тело замыкания, встроенное в [переходник][g-trampoline]. Для `double`
и `invert` переходник не нужен: в таблице их собственные адреса.

</div>

## Конструкторы и методы — тоже функции

Конструктор варианта перечисления с полями и конструктор
[кортежной структуры][g-tuple-struct] — функции, и их можно передать туда, где ждут
замыкание. Так же ведут себя методы, если назвать их полным путём:

```rust
{{#include ../../code/ch12/src/fnptr.rs:ctors}}
```

`raw.map(Reading::Raw).map(Some)` вместо `raw.map(|r| Some(Reading::Raw(r)))`
— не просто запись короче: `Reading::Raw` — тот же тип нулевого размера,
и код получается одинаковый. Clippy в группе `pedantic` предлагает эту
замену сам (`redundant_closure_for_method_calls`).

## Сравнение указателей

Два указателя на функции можно сравнить через `==`, но rustc 1.97
предупреждает:

```console
warning: function pointer comparisons do not produce meaningful results since their addresses are not guaranteed to be unique
4 | assert!(a == b);
  |         ^^^^^^
  = note: the address of the same function can vary between different codegen units
  = note: furthermore, different functions could have the same address after being merged together
```

Обе оговорки настоящие. Обобщённая функция или функция с `#[inline]`
получает копию в каждой [единице кодогенерации][g-codegen-unit], где используется, и два
указателя на «одну» функцию могут различаться. А две функции с
одинаковым машинным кодом компоновщик с `--icf` или LLVM с
`mergefunc` может слить в одну, и указатели на разные функции совпадут.
`core::ptr::fn_addr_eq` сравнивает так же, но говорит читателю, что
автор знает об этом:

```rust
{{#include ../../code/ch12/src/fnptr.rs:compare}}
```

Miri проверяет эту оговорку буквально: каждое приведение функции к
указателю он может выдать с новым адресом, и в опыте с сотней сравнений
`same(double, double)` под Miri было истинным 3 раза из 100 (в
обычной сборке — все 100). Тест `functions_as_values` под Miri
пропускает это сравнение. Указатель на функцию годится, чтобы её
вызвать, но не как ключ для поиска обработчика: для этого нужен свой
идентификатор.

## Замыкание в C и обратно

Обратный вызов в C принимает пару «указатель на функцию и `void *`», а
у замыкания тип и состояние в одном значении. Переходник между ними —
*трамплин* (trampoline): обобщённая функция с ABI C, которая знает, что за
`ctx` лежит именно `F`:

```rust
{{#include ../../code/ch12/src/fnptr.rs:tramp}}
```

`tramp::<T, F>` — своя копия на каждое замыкание, и адрес этой копии
уходит в `qsort_r` как обычный указатель на функцию. C видит пару «код
и контекст», как в введении, а Rust по-прежнему проверяет тип
контекста: перепутать `ctx` от одного замыкания с трамплином другого
нельзя, потому что пару строит одна функция `sort_with`. Замыкание
со счётчиком сравнений:

```rust
{{#include ../../code/ch12/src/fnptr.rs:sort-desc}}
```

Внутри трамплина замыкание встроено целиком:

```console
$ cargo asm -p ch12 --lib --simplify tramp
ch12::fnptr::c_sort::tramp::<u32, ch12::fnptr::c_sort::sort_desc::{closure#0}>:
	mov eax, dword ptr [rsi]
	cmp eax, dword ptr [rdi]
	seta al
	sbb al, 0
	mov rcx, qword ptr [rdx]
	inc qword ptr [rcx]
	movsx eax, al
	ret
```

Сравнение `b.cmp(a)` — четыре инструкции, `ctx` пришёл в `rdx` третьим
аргументом, и `mov rcx, qword ptr [rdx]` достаёт из структуры замыкания
её единственное поле, ссылку на `calls`, а `inc` увеличивает счётчик.
Косвенный вызов остаётся один — тот, которым `qsort_r` зовёт трамплин.

Одно отличие от C важно. [Паника][g-panic] не может пройти через кадр функции с
ABI `"C"`: если замыкание паникует внутри трамплина, процесс
завершается. Тест `panic_in_callback_aborts` запускает сам себя с
паникующим замыканием и проверяет сигнал SIGABRT и сообщение:

```console
thread 'fnptr::tests::panic_in_callback_aborts' (31380) panicked at code/ch12/src/fnptr.rs:184:38:
сравнение сломалось
…
panic in a function that cannot unwind
…
thread caused non-unwinding panic. aborting.
```

[Раскручивать][g-unwinding] стек через `msort_with_tmp` из glibc нельзя: C-код не
знает о деструкторах и не освободит свой временный буфер. ABI
`"C-unwind"` разрешает раскрутку через чужие кадры, но годится только
для C-кода, собранного с `-fexceptions` и готового к ней. Если
обработчик может паниковать, а C-код этого не переживёт, панику ловят
внутри трамплина через `std::panic::catch_unwind` и возвращают в C
код ошибки. Граница с C целиком — типы, владение, раскрутка, сборка
вместе с C-кодом — тема главы 18.

<div class="box c">

**Как в C.** GCC умеет вложенные функции с захватом — расширение, которого
нет в стандарте и в clang:

```c
{{#include ../../code/ch12/c/nested.c}}
```

Адрес `add` должен стать обычным указателем на функцию, а функции нужен
адрес кадра `main`, чтобы найти `sum`. GCC решает это тем же
трамплином, только строит его во время выполнения: кладёт на стек
несколько инструкций, которые загружают адрес кадра в регистр и
прыгают в тело `add`. Исполнять стек для этого должно быть можно:

```console
$ gcc -O2 -Wall -Wextra -Wtrampolines -o nested nested.c
nested.c:18:10: warning: trampoline generated for nested function 'add' [-Wtrampolines]
/usr/bin/ld: warning: /tmp/ccf8aS1d.o: requires executable stack (because the .note.GNU-stack section is executable)
$ ./nested
sum = 6
$ readelf -lW nested | grep GNU_STACK
  GNU_STACK      0x000000 0x0000000000000000 0x0000000000000000 0x000000 0x000000 RWE 0x10
$ clang -c nested.c
nested.c:18:21: error: function definition is not allowed here
```

`RWE` — стек, доступный для записи и исполнения сразу: ровно то, от
чего защищает флаг NX и что эксплойты переполнения буфера используют
первым делом. Трамплин Rust — обычная функция в секции кода, созданная
при компиляции, и стеку исполняемым быть не нужно. Скрипт `run.sh`
проверяет все строки этого вывода.

</div>

<div class="box check">

**Проверь себя.** Каков размер `[fn(u8) -> u8; 4]` и `[fn(u8) -> u8
{double}; 4]` на Cortex-M3? Почему `pick_fn` из следующего раздела может
вернуть одно из двух замыканий как `fn(u8) -> u8`, а замыкания с
захватом — нет? Что случится, если в `sort_with` передать в `qsort_r`
`tramp::<T, G>` для другого замыкания `G`, и почему такую ошибку
нельзя сделать, не написав `unsafe`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-bounds-check]: ../appendix-f.md#bounds-check "Проверка границ (bounds check): Сравнение индекса с длиной среза перед обращением: одна беззнаковая инструкция и переход к панике."
[g-codegen-unit]: ../appendix-f.md#codegen-unit "Единица кодогенерации (codegen unit): Часть крейта, которую LLVM оптимизирует и компилирует отдельно и параллельно с другими."
[g-faults]: ../appendix-f.md#faults "MemManage, BusFault, UsageFault: Отдельные исключения сбоев ARMv7-M: нарушение прав MPU или выполнение из запрещённой области, ошибка шины, неверная инструкция или состояние ядра (например, переход на адрес без бита Thumb)."
[g-interior-mut]: ../appendix-f.md#interior-mut "Внутренняя изменяемость (interior mutability): Изменение значения через разделяемую ссылку &."
[g-jump-table]: ../appendix-f.md#jump-table "Таблица переходов (jump table): Массив адресов или смещений веток, по которому match выполняет косвенный переход после одной проверки границы."
[g-panic]: ../appendix-f.md#panic "Паника (panic): Аварийная реакция на ошибку в самой программе: индекс за границей, unwrap на None, переполнение в отладочной сборке."
[g-trampoline]: ../appendix-f.md#trampoline "Трамплин (trampoline): Короткая функция-переходник, которая только передаёт вызов дальше, приспосабливая его к другому соглашению."
[g-tuple-struct]: ../appendix-f.md#tuple-struct "Кортежная структура (tuple struct): Структура с полями без имён: struct Rgb(u8, u8, u8), доступ через .0, .1."
[g-unwinding]: ../appendix-f.md#unwinding "Раскрутка стека (unwinding): Снятие кадров стека по одному после паники с вызовом деструкторов живых значений."
[g-vector-table]: ../appendix-f.md#vector-table "Таблица векторов (vector table): Массив слов в начале образа Cortex-M: слово 0 — начальное значение указателя стека, слово 1 — адрес обработчика сброса, дальше адреса обработчиков исключений и прерываний."
