# 4.5. noalias: что ссылки сообщают оптимизатору

Правило читателей и писателя придумано против висячих указателей. У него
есть второе следствие, о котором программисты на C обычно узнают из
профилировщика: компилятор знает, что две ссылки не указывают в одну
память. В C это знание приходится добывать правилами о типах или
обещать словом `restrict`.

## Счётчик и данные

Приёмник считает байты-флаги `0x7E` в принятом кадре HDLC и копит итог в
счётчике:

```rust
{{#include ../../code/ch04/src/noalias.rs:count}}
```

Тело цикла читает байт и прибавляет к `*cnt`. Хочется держать счётчик в
регистре и записать один раз в конце. Но можно ли? Если `cnt` указывает
внутрь `data`, каждая запись меняет байты, которые цикл прочтёт позже, и
результат зависит от того, записан ли счётчик. Для сырых указателей это
законно, и тест это показывает:

```rust
{{#include ../../code/ch04/src/noalias.rs:overlap}}
```

Для ссылок такого вызова не бывает: `&mut words[1]` и `&words` вместе —
ошибка E0502 (doc-тест в `noalias.rs`). Компилятор передаёт это LLVM
атрибутом `noalias` на параметрах:

```text
$ cargo asm -p ch04 --lib --llvm ch04_count_flags 0
define void @ch04_count_flags(ptr noalias noundef align 4 captures(none) dereferenceable(4) %cnt, ptr noalias noundef nonnull readonly captures(address) %data.0, i64 noundef range(i64 0, -9223372036854775808) %data.1) unnamed_addr #10 !dbg !9531 {
...
```

```text
$ cargo asm -p ch04 --lib --llvm ch04_count_flags_raw
define void @ch04_count_flags_raw(ptr noundef captures(none) %cnt, ptr noundef readonly captures(none) %data, i64 noundef %n) unnamed_addr #10 !dbg !9629 {
...
```

`noalias` на параметре значит: пока функция выполняется, к памяти, которую
она трогает через этот указатель, никто не обращается иначе как через
него. Кроме `noalias` из ссылки следуют `align 4`, `dereferenceable(4)` и
`nonnull`: адрес выровнен, по нему можно читать без проверки, он не ноль.

## x86-64

Со ссылками LLVM векторизует цикл: по 8 байт за итерацию, сравнение
`pcmpeqb` с вектором из `0x7E`, суммы в регистрах `xmm`. Запись счётчика в
память одна, после всех циклов:

```text
$ cargo asm -p ch04 --lib ch04_count_flags 0
ch04_count_flags:
	test rdx, rdx
	je .LBB33_9
	mov r8d, dword ptr [rdi]
...
.LBB33_4:
	movd xmm4, dword ptr [rsi + r8]
	movd xmm5, dword ptr [rsi + r8 + 4]
	pcmpeqb xmm4, xmm2
	punpcklbw xmm4, xmm4
	punpcklwd xmm4, xmm4
	pand xmm4, xmm3
	paddd xmm1, xmm4
	pcmpeqb xmm5, xmm2
	punpcklbw xmm5, xmm5
	punpcklwd xmm5, xmm5
	pand xmm5, xmm3
	paddd xmm0, xmm5
	add r8, 8
	cmp rcx, r8
	jne .LBB33_4
...
.LBB33_8:
	mov dword ptr [rdi], r8d
.LBB33_9:
	ret
```

С сырыми указателями LLVM делает две версии цикла и выбирает между ними
во время выполнения. Сначала он проверяет, пересекаются ли
`[cnt, cnt+4)` и `[data, data+n)`. Если нет — тот же векторный цикл. Если
да — скалярный, с записью `*cnt` после каждого байта:

```text
$ cargo asm -p ch04 --lib ch04_count_flags_raw
ch04_count_flags_raw:
	test rdx, rdx
	je .LBB34_11
	mov ecx, dword ptr [rdi]
	cmp rdx, 8
	jb .LBB34_2
	lea rax, [rdi + 4]
	lea r8, [rsi + rdx]
	cmp rdi, r8
	setb r8b
	cmp rsi, rax
	setb al
	test r8b, al
	je .LBB34_8
...
.LBB34_7:
	xor eax, eax
	cmp byte ptr [rsi + r8], 126
	sete al
	add eax, ecx
	mov dword ptr [rdi], eax
	xor ecx, ecx
	cmp byte ptr [rsi + r8 + 1], 126
	sete cl
	add ecx, eax
	mov dword ptr [rdi], ecx
...
	jne .LBB34_7
```

Код вдвое длиннее, и на каждом вызове — проверка. clang делает с таким же
циклом на C ровно то же. GCC 13 проверку не вставляет и векторную версию
не строит: цикл скалярный, запись на каждый байт.

```console
$ gcc -O2 -S -masm=intel count.c
```

```text
count_flags:
	endbr64
	test	rdx, rdx
	je	.L1
	mov	eax, DWORD PTR [rdi]
	add	rdx, rsi
.L3:
	xor	ecx, ecx
	cmp	BYTE PTR [rsi], 126
	sete	cl
	add	rsi, 1
	add	eax, ecx
	mov	DWORD PTR [rdi], eax
	cmp	rsi, rdx
	jne	.L3
.L1:
	ret
```

Вот C-версии, которые сравнивает `code/ch04/c/run.sh`:

```c
{{#include ../../code/ch04/c/count.c}}
```

## Cortex-M4

На Cortex-M векторных регистров нет, и проверку пересечения LLVM не
вставляет: выигрывать ей нечего. Остаётся разница в записях. Со ссылками
цикл развёрнут по 4 байта, а счётчик живёт в `r3` до самого конца:

```text
$ cargo asm -p ch04 --lib --target thumbv7em-none-eabihf ch04_count_flags 0
...
.LBB30_7:
	ldrb r4, [r2, #4]!
	ldrb r1, [r2, #1]
	cmp r4, #126
	ldrb.w r12, [r2, #2]
	ldrb r5, [r2, #3]
	it eq
	addeq r3, #1
	cmp r1, #126
	it eq
	addeq r3, #1
	cmp.w r12, #126
	it eq
	addeq r3, #1
	cmp r5, #126
	add.w r1, r2, #4
	it eq
	addeq r3, #1
	cmp r1, lr
	bne .LBB30_7
.LBB30_8:
	str r3, [r0]
...
```

С сырыми указателями после каждого байта стоит `str r3, [r0]`:

```text
$ cargo asm -p ch04 --lib --target thumbv7em-none-eabihf ch04_count_flags_raw
...
.LBB31_3:
	adds r5, r4, r2
	adds r2, #4
	ldrb r6, [r5, #3]
	cmp r6, #126
	it eq
	addeq r3, #1
	str r3, [r0]
	ldrb r6, [r5, #4]
	cmp r6, #126
	it eq
	addeq r3, #1
	str r3, [r0]
...
	bne .LBB31_3
```

Так же поступают arm-none-eabi-gcc и clang с `uint8_t`. На кадре в 1 КиБ это
1024 записи в ОЗУ вместо одной, и каждая занимает шину, которую в это
время делят с DMA.

## Как C получает то же знание

У C два способа, и оба видны в `count.c`.

Первый — *строгий алиасинг* (strict aliasing): по стандарту объект типа
`uint32_t` нельзя читать или писать через указатель на несовместимый тип.
`uint16_t` с `uint32_t` несовместимы, значит запись в `*cnt` не меняет
`data[i]`, и GCC выносит запись из цикла:

```console
$ arm-none-eabi-gcc -O2 -mcpu=cortex-m4 -mthumb -S count.c
```

```text
count_flags16:
	cbz	r2, .L7
	subs	r1, r1, #2
	ldr	r3, [r0]
	add	r2, r1, r2, lsl #1
.L9:
	ldrh	ip, [r1, #2]!
	cmp	ip, #126
	it	eq
	addeq	r3, r3, #1
	cmp	r2, r1
	bne	.L9
	str	r3, [r0]
.L7:
	bx	lr
```

Для `uint8_t` правило не работает: символьные типы (`char`, `unsigned
char`, а `uint8_t` на всех распространённых платформах — это `unsigned
char`) по стандарту могут указывать на что угодно. Как раз байтовые
буферы — главный случай в драйверах — остаются без оптимизации. Кроме
того, ядро Linux и многие прошивки собираются с `-fno-strict-aliasing`,
потому что правило слишком легко нарушить приведением указателей, и
тогда не работает и вариант с `uint16_t`.

Второй способ — `restrict`. С ним запись уходит из цикла и для `uint8_t`:

```text
count_flags_restrict:
	cbz	r2, .L14
	subs	r1, r1, #1
	ldr	r3, [r0]
	add	r2, r2, r1
.L16:
	ldrb	ip, [r1, #1]!
	cmp	ip, #126
	it	eq
	addeq	r3, r3, #1
	cmp	r2, r1
	bne	.L16
	str	r3, [r0]
.L14:
	bx	lr
```

Но `restrict` — обещание программиста, а не проверка. Тот же опыт, что в
тесте `raw_pointers_may_overlap`, на C:

```c
{{#include ../../code/ch04/c/overlap.c}}
```

```console
$ gcc -O2 overlap.c count.c && ./a.out
без restrict 0x7f, с restrict 0x7e
$ gcc -O2 -g -fsanitize=address,undefined overlap.c count.c && ./a.out
без restrict 0x7f, с restrict 0x7f
```

Версия без `restrict` дала тот же ответ, что Rust с сырыми указателями.
Версия с `restrict` вынесла запись из цикла и посчитала по старым байтам.
С санитайзерами GCC собрал код иначе, и ответы совпали, а сообщений об
ошибке нет: ни ASan, ни UBSan нарушение `restrict` не проверяют.

<div class="box c">

**Как в C.** Сводка по счётчику флагов (GCC 13, clang 18, rustc 1.97, `-O2`):

| Вариант | x86-64 | Cortex-M4 |
|---|---|---|
| Rust, `&mut u32` и `&[u8]` | векторный цикл, одна запись | одна запись |
| Rust, сырые указатели | проверка пересечения: векторный цикл или запись на каждый байт | запись на каждый байт |
| C, `uint8_t` | GCC: запись на каждый байт; clang: как Rust с сырыми указателями | запись на каждый байт |
| C, `uint16_t` | одна запись (строгий алиасинг) | одна запись |
| C, `restrict` | одна запись | одна запись |

Строки для C проверяет `code/ch04/c/run.sh`, для Rust — листинги выше.

</div>

## Обещание, которое проверяют

В Rust нарушить `noalias` можно только в `unsafe`, и тогда это
неопределённое поведение, которое Miri ловит:

```rust
{{#include ../../code/ch04/src/noalias.rs:overlap_ub}}
```

```console
$ cargo +nightly miri test -p ch04 --lib -- --ignored --exact noalias::tests::overlapping_references_are_ub
error: Undefined Behavior: trying to retag from <135719> for SharedReadWrite permission at alloc42005[0x4], but that tag does not exist in the borrow stack for this location
   --> code/ch04/src/noalias.rs:133:30
    |
133 |             ch04_count_flags(cnt, data);
    |                              ^^^ this error occurs as part of two-phase retag at alloc42005[0x4..0x8]
    |
...
help: <135719> was created by a Unique retag at offsets [0x4..0x8]
   --> code/ch04/src/noalias.rs:131:23
    |
131 |             let cnt = &mut *base.add(1);
    |                       ^^^^^^^^^^^^^^^^^
help: <135719> was later invalidated at offsets [0x0..0x8] by a SharedReadOnly retag
   --> code/ch04/src/noalias.rs:132:24
    |
132 |             let data = core::slice::from_raw_parts(base.cast::<u8>(), 8);
    |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Stacked Borrows видит ошибку в момент создания `data`: разделяемая
ссылка на все 8 байт отменила уникальную `cnt`. Tree Borrows
(`MIRIFLAGS=-Zmiri-tree-borrows`) мягче и сообщает о записи `*cnt += …` в
строке 27, пока жива `data`. Модели расходятся в деталях, но этот код
неверен в обеих.

## Где `noalias` нет

Не каждая ссылка получает `noalias`:

```rust
{{#include ../../code/ch04/src/noalias.rs:kinds}}
```

```text
$ cargo asm -p ch04 --lib --llvm ch04_na_shared
define noundef i32 @ch04_na_shared(ptr noalias noundef readonly align 4 captures(none) dereferenceable(4) %a) unnamed_addr #9 !dbg !9745 {
...
$ cargo asm -p ch04 --lib --llvm ch04_na_cell
define noundef i32 @ch04_na_cell(ptr noundef nonnull readonly align 4 captures(none) %a) unnamed_addr #9 !dbg !9707 {
...
$ cargo asm -p ch04 --lib --llvm ch04_na_pinned
define noundef i32 @ch04_na_pinned(ptr noundef nonnull readonly align 4 captures(none) %a) unnamed_addr #9 !dbg !9731 {
...
```

`&u32` — `noalias`: пока ссылка жива, значение не меняется. `&Cell<u32>` —
без него: `Cell` меняет значение через разделяемую ссылку, это
*внутренняя изменяемость* (глава [14](../ch14/05-cells.md)). Без него и `&mut` на
тип, который не `Unpin`: такие значения, как самоссылающиеся структуры из
задачи [3.D](../ch03/07-tasks.md#d-самоссылка-и-перемещение) и
асинхронные функции из главы [22](../ch22/03-pin.md), могут держать указатели
на самих себя, и `noalias` сделал бы их неопределённым поведением.

<div class="box hood">

**Под капотом.** Атрибут `noalias` для `&mut` Rust несколько раз
отключал: оптимизации LLVM, которые им пользуются, в коде на C почти не
встречали `restrict` и содержали ошибки, а Rust ставил атрибут на каждую
изменяемую ссылку и находил их первым. Окончательно атрибут вернули в
2021 году, после исправлений в LLVM 12.

</div>
