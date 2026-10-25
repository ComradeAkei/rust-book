# 3.5. Перемещение в машинном коде

## Перемещение — копия байтов, которой может не быть

По смыслу перемещение — побитовая копия представления значения: как
`memcpy` в C, после которого источник объявлен недоступным. Насколько
эта копия реальна, решает оптимизатор. Возьмём кадр в 64 байта:

```rust
{{#include ../../code/ch03/src/moves.rs:frame}}
```

```rust
{{#include ../../code/ch03/src/moves.rs:seal}}
```

64 байта не помещаются в регистры, и `ch03_seal` получает кадр по
указателю. Это видно в LLVM IR (rustc 1.97.0, x86_64):

```text
$ cargo asm -p ch03 --lib --llvm ch03_seal 0
define noundef i32 @ch03_seal(ptr dead_on_return noalias noundef align 4 captures(none) dereferenceable(64) initializes((63, 64)) %f) unnamed_addr #8 personality ptr @rust_eh_personality !dbg !7121 {
...
```

Указатель ведёт в память *вызывающего*. Атрибут `dead_on_return`
говорит, что после возврата вызывающий эту память не читает, поэтому
`ch03_seal` пишет `f.data[59]` прямо туда, без своей копии. [`noalias`][g-noalias] —
что других путей к этой памяти у функции нет.

Теперь кадр, собранный на месте и перемещённый дважды:

```rust
{{#include ../../code/ch03/src/moves.rs:move_local}}
```

```text
$ cargo asm -p ch03 --lib ch03_build_and_seal
ch03_build_and_seal:
	sub rsp, 72
	movd xmm0, edi
	punpcklbw xmm0, xmm0
	pshuflw xmm0, xmm0, 0
	pshufd xmm0, xmm0, 68
	movdqu xmmword ptr [rsp + 56], xmm0
	movdqu xmmword ptr [rsp + 44], xmm0
	movdqu xmmword ptr [rsp + 28], xmm0
	movdqu xmmword ptr [rsp + 12], xmm0
	mov dword ptr [rsp + 8], edi
	lea rdi, [rsp + 8]
	call qword ptr [rip + ch03_seal@GOTPCREL]
	add rsp, 72
	ret
```

Ни `let g = f;`, ни передача в `ch03_seal` не стоили ни одной
инструкции. Кадр строится сразу там, где его ждёт `ch03_seal`: байт `seq`
размножается в 16 байт регистра `xmm0`, четыре записи заполняют `data`
(с перекрытием на 4 байта), `seq` ложится по `[rsp + 8]`, и адрес этого
места уходит в `rdi`. На Cortex-M3 то же самое, только заполнение делает
библиотечная `__aeabi_memset4`:

```text
$ cargo asm -p ch03 --lib --target thumbv7m-none-eabi ch03_build_and_seal
ch03_build_and_seal:
	push {r4, r5, r7, lr}
	add r7, sp, #8
	sub sp, #64
	mov r4, r0
	mov r5, sp
	adds r0, r5, #4
	uxtb r2, r4
	movs r1, #60
	bl __aeabi_memset4
	mov r0, r5
	str r4, [sp]
	bl ch03_seal
	add sp, #64
	pop {r4, r5, r7, pc}
```

Иначе выходит, когда значение уже лежит где-то ещё и принадлежит не нам:

```rust
{{#include ../../code/ch03/src/moves.rs:copy_from_ref}}
```

```text
$ cargo asm -p ch03 --lib ch03_seal_copy
ch03_seal_copy:
	sub rsp, 72
	movups xmm0, xmmword ptr [rdi]
	movups xmm1, xmmword ptr [rdi + 16]
	movups xmm2, xmmword ptr [rdi + 32]
	movups xmm3, xmmword ptr [rdi + 48]
	movaps xmmword ptr [rsp + 48], xmm3
	movaps xmmword ptr [rsp + 32], xmm2
	movaps xmmword ptr [rsp + 16], xmm1
	movaps xmmword ptr [rsp], xmm0
	mov rdi, rsp
	call qword ptr [rip + ch03_seal@GOTPCREL]
	add rsp, 72
	ret
```

`ch03_seal` испортит свой экземпляр, а кадр вызывающего должен остаться
целым, поэтому 64 байта копируются в кадр стека: четыре чтения и четыре
записи по 16 байт. На Cortex-M3 копию делают групповые инструкции
`ldm`/`stm`, по 5–6 слов за раз:

```text
$ cargo asm -p ch03 --lib --target thumbv7m-none-eabi ch03_seal_copy
ch03_seal_copy:
	push {r4, r5, r6, r7, lr}
	add r7, sp, #12
	str r11, [sp, #-4]!
	sub sp, #64
	ldm.w r0!, {r1, r3, r4, r5, lr}
	mov r12, sp
	mov r2, r12
	stm.w r2!, {r1, r3, r4, r5, lr}
	ldm.w r0!, {r1, r3, r4, r5, lr}
	stm.w r2!, {r1, r3, r4, r5, lr}
	ldm.w r0, {r1, r3, r4, r5, r6, lr}
	mov r0, r12
	stm.w r2, {r1, r3, r4, r5, r6, lr}
	bl ch03_seal
	add sp, #64
	ldr r11, [sp], #4
	pop {r4, r5, r6, r7, pc}
```

Правило: перемещение бесплатно, когда компилятор может построить
значение сразу на его последнем месте. Оно стоит копии, когда значение
уже существует в другом месте. Маленькие значения при этом вообще не
трогают память и едут в регистрах. Где проходит граница «в регистрах или
по указателю» для Rust [ABI][g-abi] и для `extern "C"`, на x86_64 и на Cortex-M,
и с какого размера копия превращается в вызов `memcpy`, вы выясните сами
в задаче [3.C](07-tasks.md#c-цена-перемещения).

## Сборка по значению и через &mut

Блок журнала для записи во флеш занимает 1 КиБ. Его можно собирать в
стиле «значение входит, значение выходит» или через `&mut`:

```rust
{{#include ../../code/ch03/src/moves.rs:builder}}
```

Каждый шаг сборки по значению получает блок по указателю на память
вызывающего и возвращает его через скрытый указатель на место для
результата (`sret`):

```text
$ cargo asm -p ch03 --lib --llvm --target thumbv7m-none-eabi with_header
define dso_local void @ch03::moves::with_header(ptr dead_on_unwind noalias noundef writable writeonly sret([1024 x i8]) align 4 captures(none) dereferenceable(1024) initializes((0, 1024)) %_0, ptr dead_on_return noalias noundef align 4 captures(none) dereferenceable(1024) initializes((0, 4)) %b, i32 noundef %seq) unnamed_addr #2 !dbg !246 {
...
```

Аргумент и результат — разные места в памяти, и шаг кончается
копированием всего блока из одного в другое (Cortex-M3):

```text
$ cargo asm -p ch03 --lib --target thumbv7m-none-eabi with_header
ch03::moves::with_header:
	push {r7, lr}
	mov r7, sp
	str r2, [r1]
	mov.w r2, #1024
	bl __aeabi_memcpy4
	pop {r7, pc}
```

`str r2, [r1]` записывает `seq` в заголовок аргумента, а
`__aeabi_memcpy4` копирует 1024 байта из аргумента (`r1`) в результат
(`r0`). `with_payload` и `with_crc` кончаются так же. Сама
`build_by_value` держит на стеке два блока и гоняет значение между ними:

```text
$ cargo asm -p ch03 --lib --target thumbv7m-none-eabi build_by_value
ch03::moves::build_by_value:
	push {r4, r5, r6, r7, lr}
	add r7, sp, #12
	push.w {r8, r9, r11}
	sub.w sp, sp, #2048
	...
	bl __aeabi_memclr4
	...
	bl ch03::moves::with_header
	...
	bl ch03::moves::with_payload
	mov r0, r8
	mov r1, r4
	bl ch03::moves::with_crc
	add.w sp, sp, #2048
	pop.w {r8, r9, r11}
	pop {r4, r5, r6, r7, pc}
```

Сборка одного блока стоит трёх копий по килобайту и 2 КиБ стека. Сборка
через `&mut` не копирует ничего: `build_in_place` передаёт дальше один
указатель, а `set_header` — это одна запись `str r1, [r0]`.

Сколько стека это стоит на деле, меряет прошивка `firmware/ch03-stack`
*закраской стека* (stack painting).
Перед вызовом она закрашивает свободную часть стека известным словом, а
после ищет самый низкий адрес, где краска затёрта:

```rust
{{#include ../../firmware/ch03-stack/src/lib.rs:symbols}}
```

```rust
{{#include ../../firmware/ch03-stack/src/lib.rs:paint}}
```

```rust
{{#include ../../firmware/ch03-stack/src/bin/stack.rs:main}}
```

```console
$ cd firmware
$ cargo run --release -p ch03-stack --bin stack
стек: 0x20000008..0x20004000, 16376 байт
по значению: crc = 0x2871b, стек 2188 байт
через &mut:  crc = 0x2871b, стек 76 байт
```

Результат один, но сборка по значению заняла почти в 29 раз больше
стека: 2188 байт против 76. Это 13 % от 16 КиБ ОЗУ платы. На хосте с
мегабайтами стека разницу не заметить, на микроконтроллере она решает,
поместится ли программа. Большие значения на МК передают через `&mut` и
держат в статических буферах, а стиль «значение входит, значение
выходит» оставляют для того, что помещается в регистры.

<div class="box ref">

**Справка: стек на Cortex-M.** Первое слово [таблицы векторов][g-vector-table] — начальное
значение указателя стека `MSP`, при сбросе ядро загружает его само.
`cortex-m-rt` кладёт туда адрес конца ОЗУ (символ `_stack_start`), и стек
растёт вниз, навстречу секциям `.data` и `.bss` в начале ОЗУ. Нижняя
граница стека (`_stack_end`) — просто конец `.bss`: в прошивке `stack` это
`0x20000008`, потому что первые 8 байт ОЗУ занимает статическая
переменная `HSTDOUT` из `cortex-m-semihosting`. Ни MMU, ни [страницы-сторожа][g-guard-page]
нет: ниже границы стек продолжает писать в статические переменные, как
показано ниже.

</div>

## Box::new и большой массив

```rust
{{#include ../../code/ch03/examples/big_box.rs:big_box}}
```

`Box::new` — обычная функция, а её аргумент — обычное значение. По
смыслу массив в 16 МиБ сначала строится на стеке, а потом перемещается в
кучу. Без оптимизаций так и происходит, и главный поток с его 8 МиБ стека
(`ulimit -s` в Linux по умолчанию) падает:

```console
$ cargo run -p ch03 --example big_box
thread 'main' (16006) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

В release LLVM видит, что массив нулевой, строит его сразу в куче и
вызывает `__rust_alloc_zeroed`:

```console
$ cargo run --release -p ch03 --example big_box
выделено 16 МиБ по адресу 0x7f70af7ff010
```

Полагаться на оптимизатор здесь нельзя. Большой буфер в куче правильно
создавать функциями, которые выделяют память сразу:
`vec![0u8; N].into_boxed_slice()` или `Box::<[u8; N]>::new_zeroed()`
(второй вариант возвращает `Box<MaybeUninit<_>>`, и `assume_init` на нём
небезопасен).

<div class="box ref">

**Справка: переполнение стека в Linux.** Под стеком каждого потока ядро
или `pthread` оставляют *страницу-сторож* (guard page) без прав доступа. Обращение к ней
даёт `SIGSEGV`. Среда выполнения Rust при старте ставит обработчик
`SIGSEGV` на отдельном стеке (`sigaltstack`). Если адрес ошибки попал в
страницу-сторож, обработчик печатает сообщение о переполнении вместо
безымянного «Segmentation fault» и завершает процесс. У программы на C сообщения не будет, но и
повреждения памяти тоже: страница-сторож останавливает процесс.

</div>

## Переполнение без страницы-сторожа

На микроконтроллере страницы-сторожа нет. Прошивка `overflow` кладёт
под стек две статические переменные и уходит в рекурсию на 64 байта
ниже границы стека:

```rust
{{#include ../../firmware/ch03-stack/src/bin/overflow.rs:statics}}
```

```rust
{{#include ../../firmware/ch03-stack/src/lib.rs:dig}}
```

```rust
{{#include ../../firmware/ch03-stack/src/bin/overflow.rs:main}}
```

```console
$ cargo run --release -p ch03-stack --bin overflow
SETTINGS 0x20000000, COUNTERS 0x20000100, стек 0x20000200..0x20004000
до: испорчено SETTINGS 0/64, COUNTERS 0/64, COUNTERS[63] = 0x00000000
рекурсия: 56 уровней по 256 байт
после: испорчено SETTINGS 0/64, COUNTERS 41/64, COUNTERS[63] = 0xeeeeeeee
```

Исключения нет, прошивка работает дальше и завершается успешно. 41
слово из 64 счётчиков теперь содержит байты `0xEE` из буферов рекурсии и
сохранённые регистры. Программа на C на том же ядре повела бы себя точно
так же: Rust не проверяет глубину стека, и безопасный код здесь ничем не
защищён от порчи памяти. Это одно из немногих мест, где гарантии языка
опираются на платформу, а на голом железе платформа их не даёт.

<div class="box trap">

**Ловушка.** Первая версия этой прошивки печатала через `hprintln!` и
зависала после рекурсии. Причина нашлась в карте памяти: `hprintln!`
хранит открытый поток вывода в статической переменной `HSTDOUT` из
`cortex-m-semihosting`, а та лежала в `.bss` прямо под стеком, и
рекурсия её затёрла. Переполнение портит не только ваши переменные, но и
внутреннее состояние библиотек, и проявляется далеко от места причины.
Прошивка `overflow` печатает макросом, который открывает поток заново
при каждом вызове:

```rust
{{#include ../../firmware/ch03-stack/src/bin/overflow.rs:say}}
```

</div>

Если стек дойдёт до начала ОЗУ, запись по адресу ниже `0x20000000`
уйдёт в пустоту. На кристалле это ошибка шины. Модель QEMU такие записи
молча отбрасывает, а чтения оттуда дают ноль (раздел
[27.7](../ch27/07-faults.md)), и сбой приходит позже — по последствиям.
Прошивка `lockup` делает рекурсию без границы, и журнал QEMU с ключом
`-d int,guest_errors` показывает всю цепочку.

Первым за границу уходит адрес возврата: его кладёт на стек
`__aeabi_memset4`, которая заполняет буфер очередного кадра. Возврат из
неё читает этот адрес обратно — ноль — и переходит по адресу 0 со
сброшенным битом [Thumb][g-thumb]: [UsageFault][g-faults] INVSTATE. UsageFault после сброса
выключен, и сбой эскалирует в `HardFault`. Чтобы войти в обработчик,
ядро кладёт восемь регистров на тот же стек, за пределы памяти, и
получает ошибку шины STKERR, но в обработчик всё же входит. Обработчик
у прошивки есть:

```rust
{{#include ../../firmware/ch03-stack/src/bin/lockup.rs:hardfault}}
```

Только стека нет и у него. `ef` указывает на кадр, которого нет, и
читается нулями. `hprintln!` передаёт semihosting блок параметров на
стеке — дескриптор, адрес и длину строки, — и оттуда тоже читаются нули,
так что печатать нечего. Первый же возврат из функции снова берёт со
стека ноль и снова прыгает на адрес 0. Это INVSTATE уже при выполнении
`HardFault`, эскалировать его некуда, и ядро входит в состояние
*lockup* — перестаёт выполнять инструкции: для сбоя в самом
`HardFault` (или в *NMI*, non-maskable interrupt — немаскируемом
прерывании) архитектура обработчика не предусматривает. На
кристалле цепочка начинается раньше, с первой же записи за границу, но
кончается так же: сбоем внутри `HardFault` и lockup.

```console
$ cargo run --release -p ch03-stack --bin lockup
рекурсия без границы
qemu: fatal: Lockup: can't escalate 3 to HardFault (current priority -1)

R00=00000001 R01=fffffffe R02=1fffff0c R03=20000054
R04=00000000 R05=00000000 R06=00000000 R07=00000000
R08=00000000 R09=00000000 R10=00000000 R11=00000000
R12=00000000 R13=1fffff08 R14=00000585 R15=00000000
XPSR=a0000003 N-C- A handler
```

`R13` — указатель стека, `0x1fffff08`, ниже начала ОЗУ. Младшие биты
`XPSR` равны 3: ядро уже в исключении номер 3, `HardFault`. `R15` —
ноль, тот самый адрес 0, а бит T (24) в `XPSR` сброшен: это INVSTATE.
`R14` = 0x585 — адрес возврата из `core::fmt::write`, которую вызвал
`hprintln!` в обработчике. QEMU останавливается с кодом 134. Настоящий
кристалл остаётся в lockup, пока его не сбросит сторожевой таймер или
отладчик (выйти из lockup может и NMI).

Lockup лучше тихой порчи: устройство останавливается и перезагружается,
а не работает на испорченных данных. На этом построены две защиты.
Инструмент компоновки [flip-link][g-flip-link] переставляет стек в начало ОЗУ, под
`.data` и `.bss`, чтобы переполнение упиралось в границу памяти, а не в
переменные. Блок защиты памяти [MPU][g-mpu] (необязательная часть Cortex-M3, M4 и
M0+, у Cortex-M0 его нет) позволяет запретить запись в полосу под
стеком, и выход за неё вызывает `MemManage`. Измерять же расход стека
можно закраской, как в прошивке `stack`.

<div class="box check">

**Проверь себя.**

1. Почему `ch03_build_and_seal` не копирует кадр, а `ch03_seal_copy`
   копирует? Что изменится, если `ch03_seal` будет принимать `&Frame`?
2. Сборка по значению заняла 2188 байт стека. Из чего складывается эта
   цифра, если два блока занимают 2048?
3. Почему прошивка `overflow` завершилась успешно, а не упала? Какой
   код должен был бы проверять глубину стека и почему его нет?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-abi]: ../appendix-f.md#abi "ABI (application binary interface): Двоичный интерфейс платформы: соглашение о вызовах, раскладка типов в памяти, имена символов."
[g-faults]: ../appendix-f.md#faults "MemManage, BusFault, UsageFault: Отдельные исключения сбоев ARMv7-M: нарушение прав MPU или выполнение из запрещённой области, ошибка шины, неверная инструкция или состояние ядра (например, переход на адрес без бита Thumb)."
[g-flip-link]: ../appendix-f.md#flip-link "flip-link: Обёртка над компоновщиком для Cortex-M: кладёт стек в начало ОЗУ, под .data и .bss, и переполненный стек упирается в границу памяти и вызывает сбой, а не портит статические переменные."
[g-guard-page]: ../appendix-f.md#guard-page "Страница-сторож (guard page): Страница без прав доступа под стеком потока: переполнение стека упирается в неё и даёт SIGSEGV, а не портит соседнюю память."
[g-mpu]: ../appendix-f.md#mpu "MPU (memory protection unit): Блок защиты памяти Cortex-M: несколько областей с правами доступа, нарушение которых вызывает MemManage."
[g-noalias]: ../appendix-f.md#noalias "noalias: Атрибут LLVM на параметре-указателе: пока функция выполняется, к памяти, которую она трогает через этот указатель, никто не обращается иначе как через него. rustc ставит его на &mut T и &T, кроме ссылок на типы с внутренней изменяемостью и &mut на типы, которые не Unpin."
[g-thumb]: ../appendix-f.md#thumb "Thumb, Thumb-2: Набор команд Cortex-M."
[g-vector-table]: ../appendix-f.md#vector-table "Таблица векторов (vector table): Массив слов в начале образа Cortex-M: слово 0 — начальное значение указателя стека, слово 1 — адрес обработчика сброса, дальше адреса обработчиков исключений и прерываний."
