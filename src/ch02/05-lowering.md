# 2.5. Во что компилируется match

`match` по целому числу или по перечислению без данных rustc передаёт
LLVM в виде одной инструкции `switch` [промежуточного представления][g-ir], той
же, в которую clang превращает `switch` из C. Дальше выбор за LLVM, и
способов у него пять:

| Способ | Когда | Пример |
|---|---|---|
| таблица переходов | много веток с разным кодом, значения плотные | `ch02_exec` |
| таблица значений | каждая ветка только возвращает константу | `ch02_segments` |
| битовая маска | результат `bool`, значения укладываются в 64 бита | `ch02_is_space` |
| формула | константы ложатся на прямую | `ch02_adc_delay` |
| дерево сравнений | значения разрежены | `ch02_service` |

Все функции раздела лежат в `code/ch02/src/lowering.rs` с атрибутами
`#[inline(never)]` и `#[unsafe(no_mangle)]`. Без них маленькая
функция [встроилась][g-inlining] бы в вызывающий код, а то и вовсе не попала бы в
объектный файл библиотеки, и `cargo asm` её бы не нашёл.

## Таблица переходов

Восемь операций виртуальной машины, у каждой свой код:

```rust
{{#include ../../code/ch02/src/lowering.rs:jump_table}}
```

LLVM получает от rustc такой `switch` (строки отладочной информации
убраны):

```text
$ cargo asm -p ch02 --lib --llvm ch02_exec 0
define noundef i32 @ch02_exec(i8 noundef %op, i32 noundef %a, i32 noundef %b) unnamed_addr #10 {
start:
  switch i8 %op, label %bb10 [
    i8 0, label %bb9
    i8 1, label %bb8
    i8 2, label %bb7
    i8 3, label %bb6
    i8 4, label %bb5
    i8 5, label %bb4
    i8 6, label %bb3
    i8 7, label %bb2
  ]
```

и превращает его в косвенный переход по *таблице переходов* (jump table) —
массиву смещений веток:

```text
$ cargo asm -p ch02 --lib ch02_exec 0
ch02_exec:
	cmp dil, 7
	ja .LBB28_1
	mov eax, edx
	movzx ecx, dil
	lea rdx, [rip + .LJTI28_0]
	movsxd rcx, dword ptr [rdx + 4*rcx]
	add rcx, rdx
	jmp rcx
	add eax, esi
	ret
	or eax, esi
	ret
	imul eax, esi
	ret
	and eax, esi
	ret
	mov ecx, eax
	shr esi, cl
	mov eax, esi
	ret
	sub esi, eax
	mov eax, esi
	ret
	xor eax, esi
	ret
	mov ecx, eax
	shl esi, cl
	mov eax, esi
	ret
.LBB28_1:
	xor eax, eax
	ret
```

Сначала проверка границы: всё, что больше 7, уходит на ветку `_`. Затем
из таблицы `.LJTI28_0` читается 32-битное смещение ветки относительно
начала таблицы, к нему прибавляется адрес таблицы, и `jmp rcx` переходит
туда. Смещения, а не адреса, делают код независимым от адреса загрузки
(position-independent code, *PIC*): таблице не нужны релокации
(relocations) — правки, которыми загрузчик вписывает настоящие адреса при
запуске. Ветки идут не по порядку
`op`, а так, как их разложил LLVM; `cargo asm` не печатает метки, на
которые ссылается только таблица.

На Cortex-M3 для этого есть инструкция:

```text
$ cargo asm -p ch02 --lib --target thumbv7m-none-eabi ch02_exec 0
ch02_exec:
	push {r7, lr}
	mov r7, sp
	uxtb r0, r0
	cmp r0, #7
	bhi .LBB13_11
.LCPI13_0:
	tbb [pc, r0]
	.byte	(.LBB13_3-(.LCPI13_0+4))/2
	.byte	(.LBB13_8-(.LCPI13_0+4))/2
	...
```

`tbb [pc, r0]` (*table branch byte*) читает байт из таблицы, которая
лежит сразу за инструкцией, удваивает его и прибавляет к `pc`. Таблица
из восьми байт вместо 32 на x86_64, и переход занимает одну инструкцию.
Для дальних веток есть `tbh` с 16-битными элементами. У Cortex-M0
([ARMv6-M][g-armv7m]) таких инструкций нет, и LLVM собирает то же из четырёх:

```text
$ cargo asm -p ch02 --lib --target thumbv6m-none-eabi ch02_exec 0
ch02_exec:
	push {r7, lr}
	add r7, sp, #0
	uxtb r0, r0
	cmp r0, #7
	bhi .LBB13_11
	add r0, pc
	ldrb r0, [r0, #4]
	lsls r0, r0, #1
.LCPI13_0:
	add pc, r0
	.byte	(.LBB13_3-(.LCPI13_0+4))/2
	...
```

<div class="box ref">

**Справка: `pc` в Thumb.** Инструкция, которая читает `pc`, видит адрес
самой себя плюс 4: так устроен конвейер ранних ARM, и архитектура
сохранила это правило. Поэтому смещения в таблице отсчитываются от
`.LCPI13_0+4`. На Cortex-M0 `add r0, pc` стоит на 8 байт раньше таблицы
и получает адрес на 4 байта меньше её начала; `ldrb r0, [r0, #4]`
добирает эти 4 байта. `add pc, r0` — запись в `pc`, то есть переход.

</div>

## Таблица значений

Если каждая ветка только возвращает константу, переход не нужен вовсе:
достаточно прочитать ответ из массива.

```rust
{{#include ../../code/ch02/src/lowering.rs:lookup_table}}
```

```text
$ cargo asm -p ch02 --lib ch02_segments
ch02_segments:
	cmp dil, 9
	ja .LBB32_1
	movzx eax, dil
	lea rcx, [rip + .Lswitch.table.ch02_segments]
	movzx eax, byte ptr [rax + rcx]
	ret
.LBB32_1:
	xor eax, eax
	ret

$ cargo asm -p ch02 --lib --target thumbv6m-none-eabi ch02_segments
ch02_segments:
	push {r7, lr}
	add r7, sp, #0
	uxtb r0, r0
	cmp r0, #9
	bhi .LBB17_2
	ldr r1, .LCPI17_0
	ldrb r0, [r1, r0]
	pop {r7, pc}
.LBB17_2:
	movs r0, #0
	pop {r7, pc}
.LCPI17_0:
	.long	.Lswitch.table.ch02_segments
```

`.Lswitch.table.ch02_segments` — десять байт в `.rodata` (на
микроконтроллере это флеш-память). Cortex-M0 не умеет загрузить 32-битный
адрес непосредственным операндом, поэтому адрес таблицы лежит рядом с
кодом, в *литеральном пуле* (literal pool) `.LCPI17_0`, и читается `ldr` относительно
`pc`. Cortex-M3 собирает адрес двумя инструкциями `movw` и `movt`.

## Битовая маска

Если `match` отвечает «да или нет», таблицей может служить одно число:

```rust
{{#include ../../code/ch02/src/lowering.rs:bitmask}}
```

```text
$ cargo asm -p ch02 --lib ch02_is_space
ch02_is_space:
	mov ecx, edi
	cmp cl, 33
	setb dl
	movabs rax, 4294977024
	shr rax, cl
	and al, dl
	ret
```

4294977024 — это `0x1_0000_2600`: биты 9 (`\t`), 10 (`\n`), 13 (`\r`) и
32 (пробел). Сдвиг константы на значение байта ставит нужный бит в
младший разряд, а `cmp cl, 33` отсекает байты больше 32: сдвиг на 64 и
больше x86 выполняет по модулю 64. Ни одного перехода. На Cortex-M3 LLVM
сначала вычитает 9, чтобы маска поместилась в 32-битный регистр:

```text
$ cargo asm -p ch02 --lib --target thumbv7m-none-eabi ch02_is_space
ch02_is_space:
	push {r7, lr}
	mov r7, sp
	subs r0, #9
	movs r2, #19
	movt r2, #128
	movs r1, #0
	uxtb r0, r0
	cmp r0, #24
	lsr.w r0, r2, r0
	it lo
	movlo r1, #1
	ands r0, r1
	pop {r7, pc}
```

Маска `0x0080_0013` (`movt` кладёт 128 в старшую половину, `movs` 19 в
младшую) — те же четыре символа, сдвинутые на 9: биты 0, 1, 4 и 23.

## Формула

Константы на прямой `10 + 3·m` LLVM распознаёт и заменяет таблицу
вычислением:

```rust
{{#include ../../code/ch02/src/lowering.rs:linear}}
```

```text
$ cargo asm -p ch02 --lib ch02_adc_delay
ch02_adc_delay:
	xor ecx, ecx
	cmp dil, 5
	movzx eax, dil
	lea eax, [rax + 2*rax + 10]
	cmovae eax, ecx
	ret

$ cargo asm -p ch02 --lib --target thumbv7m-none-eabi ch02_adc_delay
ch02_adc_delay:
	push {r7, lr}
	mov r7, sp
	uxtb r1, r0
	movs r0, #0
	add.w r2, r1, r1, lsl #1
	cmp r1, #5
	it lo
	addlo.w r0, r2, #10
	pop {r7, pc}
```

`lea eax, [rax + 2*rax + 10]` — это и есть `3·m + 10` одной инструкцией,
а ветка `_` стала условной пересылкой `cmovae`. На Cortex-M3 сдвиг
второго операнда (`r1, lsl #1`) бесплатен, а условное сложение `addlo`
выполняется под `it lo`.

## Дерево сравнений

Значения, разбросанные по диапазону, в таблицу не уложить: для портов от
21 до 8080 она заняла бы 8060 элементов ради семи полезных.

```rust
{{#include ../../code/ch02/src/lowering.rs:compare_tree}}
```

```text
$ cargo asm -p ch02 --lib ch02_service
ch02_service:
	movzx ecx, di
	xor eax, eax
	cmp ecx, 79
	jle .LBB33_1
	cmp ecx, 1882
	jg .LBB33_9
	cmp ecx, 80
	je .LBB33_14
	cmp ecx, 443
	jne .LBB33_16
	mov al, 5
	ret
.LBB33_1:
	cmp ecx, 21
	je .LBB33_12
	cmp ecx, 22
	je .LBB33_13
	cmp ecx, 23
	jne .LBB33_16
	mov al, 3
	ret
.LBB33_9:
	cmp ecx, 1883
	je .LBB33_15
	cmp ecx, 8080
	jne .LBB33_16
	mov al, 7
	ret
	...
```

Первое сравнение делит значения пополам (до 79 и от 80), второе — правую
половину ещё раз. Получается двоичный поиск: на семь значений не больше
четырёх сравнений вместо семи подряд.

Где проходит граница между таблицей и сравнениями, решают эвристики LLVM:
число веток, плотность значений и то, оптимизируется ли код по скорости
или по размеру. Найти эти пороги опытом — задача A
(раздел [2.7](07-tasks.md)).

<div class="box hood">

**Под капотом.** Таблицы переходов можно запретить: `-C jump-tables=no`
отключает и таблицы переходов, и таблицы значений. `ch02_exec` тогда
превращается в дерево из трёх уровней: `cmp ecx, 3; jg`, `cmp ecx, 1; jg`,
`test ecx, ecx; je`. Ядро Linux на x86 в некоторых конфигурациях защиты от
атак на спекулятивное выполнение собирается без таблиц переходов:
косвенный переход — удобная точка для такой атаки. Цена этого решения
видна в следующем разделе.

</div>

## Предсказатель переходов

Современный процессор не ждёт, пока вычислится адрес перехода: он
угадывает его и выполняет команды наперёд, а при ошибке выбрасывает
сделанное. Таблица переходов — косвенный переход, и угадать его можно,
только если коды операций идут в предсказуемом порядке. Замер прогоняет
одну и ту же функцию `ch02_exec` по трём последовательностям кодов
одинаковой длины:

```rust
{{#include ../../code/ch02/examples/dispatch.rs:dispatch}}
```

```console
$ cargo run -p ch02 --release --example dispatch
всегда 0                1.86 нс на операцию
0, 1, …, 7 по кругу     2.46 нс на операцию
случайные 0–7          10.55 нс на операцию
```

Тот же замер с `RUSTFLAGS="-C jump-tables=no"`, то есть с деревом
сравнений вместо таблицы:

| Последовательность | таблица переходов, нс | дерево сравнений, нс |
|---|---|---|
| всегда 0 | 1,87 | 1,61 |
| 0, 1, …, 7 по кругу | 2,46 | 3,23 |
| случайные 0–7 | 10,59 | 12,71 |

Это медианы трёх запусков на Intel Xeon 2,8 ГГц в облачной виртуальной
машине. В первых двух строках отдельные запуски расходятся почти на две
наносекунды, в третьей — меньше чем на полнаносекунды.

Количество работы во всех строках одинаковое. Повторяющийся код и цикл
из восьми кодов предсказатель запоминает, и операция занимает две-три
наносекунды. Случайный код он угадывает в одном случае из восьми, и
каждая ошибка стоит двух-трёх десятков тактов. Дерево сравнений на
случайных кодах ещё хуже: в нём три условных перехода на операцию, и
каждый угадывается в среднем в половине случаев. Зато на постоянном коде
дерево чуть быстрее таблицы.

<div class="box ref">

**Справка: предсказание переходов.** Процессор x86_64 держит таблицу
адресов недавних переходов (BTB) и историю направлений; для косвенных
переходов он учитывает и путь, которым пришёл к инструкции, поэтому
выучивает и цикл из восьми разных адресов. Ошибка стоит столько тактов,
сколько стадий конвейера нужно заполнить заново.

У Cortex-M0 и Cortex-M3 короткий конвейер из трёх стадий и нет
динамического предсказателя: взятый переход стоит фиксированные
несколько тактов на перезаполнение конвейера, их число указано в
справочнике ядра (*Technical Reference Manual*, раздел о времени
выполнения инструкций). Время работы `match` на этих ядрах не зависит от
порядка данных, и для обработчика прерывания с жёстким сроком это
достоинство. Предсказатель появляется в Cortex-M7 и в ядрах Cortex-A.

</div>

## Холодные ветки: #[cold] и cold_path

LLVM раскладывает код так, чтобы частый путь шёл без переходов, и
распределяет регистры в пользу частого пути. Какой путь частый, он
угадывает сам, но ему можно подсказать. Атрибут `#[cold]` на функции
говорит, что вызывается она редко:

```rust
{{#include ../../code/ch02/src/lowering.rs:cold}}
```

Две функции отличаются только атрибутом у обработчика, а код разный:

```text
$ cargo asm -p ch02 --lib ch02_sum_data 0
ch02_sum_data:
	push r15
	push r14
	push rbx
	test rsi, rsi
	je .LBB34_1
	mov r14, rsi
	mov r15, rdi
	add r14, rdi
	xor ebx, ebx
.LBB34_3:
	movzx edi, byte ptr [r15]
	cmp edi, 239
	jbe .LBB34_4
	inc r15
	call ch02::lowering::note_dropped
	cmp r15, r14
	jne .LBB34_3
	jmp .LBB34_6
.LBB34_4:
	add ebx, edi
	inc r15
	cmp r15, r14
	jne .LBB34_3
	jmp .LBB34_6
	...

$ cargo asm -p ch02 --lib ch02_sum_data_cold
ch02_sum_data_cold:
	test rsi, rsi
	je .LBB35_1
	push rbp
	push r14
	push rbx
	add rsi, rdi
	xor eax, eax
.LBB35_4:
	movzx ecx, byte ptr [rdi]
	cmp ecx, 239
	ja .LBB35_6
	add eax, ecx
	inc rdi
	cmp rdi, rsi
	jne .LBB35_4
	jmp .LBB35_7
.LBB35_6:
	inc rdi
	mov rbx, rdi
	mov edi, ecx
	mov ebp, eax
	mov r14, rsi
	call ch02::lowering::note_dropped_cold
	mov rdi, rbx
	mov rsi, r14
	mov eax, ebp
	cmp rbx, r14
	jne .LBB35_4
	...
```

Без подсказки частым LLVM счёл путь с вызовом: он идёт сразу за
сравнением, а сложение — по переходу `jbe`. Сумма, указатель и конец
среза весь цикл живут в регистрах `rbx`, `r15`, `r14`, которые
сохраняются при вызове функции, и ради этого их приходится сохранять в
стек на входе, даже для пустого среза. С `#[cold]` частый путь идёт
подряд, без взятых переходов, кроме перехода в начало цикла. Значения
живут в рабочих регистрах `eax`, `rdi`, `rsi`, а редкая ветка `.LBB35_6`
вынесена за цикл и сама перекладывает их в сохраняемые регистры вокруг
вызова.
Пустой срез проверяется до сохранения регистров (*shrink-wrapping*).

Когда в редкой ветке нет отдельной функции, её отмечают вызовом
`core::hint::cold_path()` (стабилен с Rust 1.95):

```rust
{{#include ../../code/ch02/src/lowering.rs:cold_path}}
```

Частый путь снова идёт подряд, а увеличение счётчика `*dropped` вынесено
в ветку `.LBB27_7` после цикла:

```text
$ cargo asm -p ch02 --lib ch02_count_data
ch02_count_data:
	test rsi, rsi
	je .LBB27_1
	add rsi, rdi
	mov ecx, dword ptr [rdx]
	xor eax, eax
.LBB27_4:
	movzx r8d, byte ptr [rdi]
	inc rdi
	cmp r8d, 239
	ja .LBB27_7
	add eax, r8d
	cmp rdi, rsi
	jne .LBB27_4
	jmp .LBB27_2
.LBB27_7:
	inc ecx
	mov dword ptr [rdx], ecx
	cmp rdi, rsi
	jne .LBB27_4
	jmp .LBB27_2
	...
```

<div class="box c">

**Как в C.** В C ту же подсказку дают `__builtin_expect` (в ядре Linux
это макросы `likely` и `unlikely`) и `__attribute__((cold))`. В
стабильном Rust `likely` и `unlikely` нет, но их пишут через
`cold_path()`, как показано в документации `core::hint::cold_path`.
Пути к панике rustc помечает холодными сам: функции паники стандартной
библиотеки объявлены с `#[cold]`, поэтому проверки границ и переполнения
почти не мешают частому пути.

</div>

## assert_unchecked: обещание вместо проверки

Таблица переходов начинается с проверки границы: `cmp dil, 7; ja`. Если
вызывающий код гарантирует, что `op < 8`, проверку можно убрать,
пообещав это компилятору:

```rust
{{#include ../../code/ch02/src/lowering.rs:assume}}
```

```text
$ cargo asm -p ch02 --lib ch02_exec_assume
ch02_exec_assume:
	mov eax, edx
	movzx ecx, dil
	lea rdx, [rip + .LJTI29_0]
	movsxd rcx, dword ptr [rdx + 4*rcx]
	add rcx, rdx
	jmp rcx
	...
```

Ветка `_` исчезла вместе с проверкой, а на Cortex-M3 пропали `cmp r0, #7`
и `bhi`. `assert_unchecked` (стабилен с Rust 1.81) — `unsafe`: это не
проверка, а обещание. Если оно нарушено, `jmp` возьмёт смещение из памяти
за концом таблицы и уйдёт по случайному адресу. Нарушим его нарочно:

```rust
{{#include ../../code/ch02/examples/unreachable.rs:unreachable}}
```

```console
$ cargo run -p ch02 --example unreachable

thread 'main' (13719) panicked at code/ch02/src/lowering.rs:104:14:
unsafe precondition(s) violated: hint::assert_unchecked must never be called when the condition is false

This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
thread caused non-unwinding panic. aborting.
$ echo $?
134

$ cargo run -p ch02 --release --example unreachable
$ echo $?
139

$ cargo +nightly miri run -p ch02 --example unreachable
error: Undefined Behavior: `assume` called with `false`
   --> code/ch02/src/lowering.rs:104:14
    |
104 |     unsafe { core::hint::assert_unchecked(op < 8) };
    |              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
```

Строки, которыми оболочка сообщает о сигнале, здесь опущены. В
отладочной сборке стандартная библиотека проверяет обещание и аварийно
завершает программу (код 134 — `SIGABRT`). Сообщение само
предупреждает, что проверка необязательна. В release проверки нет, и
программа падает по `SIGSEGV` (код 139), не напечатав ни строки: переход
ушёл в никуда. Могло быть и хуже — переход в чужой код с продолжением
работы. Miri находит нарушение в любом случае (раздел
[0.5](../ch00/05-checks.md)).

<div class="box trick">

**Трюк: ограничить значение без `unsafe`.** Часто диапазон можно сделать
очевидным для компилятора. Маска `op & 7` гарантирует значение от 0 до 7,
и ветка `_` становится недостижимой:

```rust
{{#include ../../code/ch02/src/lowering.rs:masked}}
```

```text
$ cargo asm -p ch02 --lib ch02_exec_masked
ch02_exec_masked:
	mov eax, edx
	and edi, 7
	lea rcx, [rip + .LJTI30_0]
	...
```

Проверка границы заменилась одной инструкцией `and`, а `unreachable!()`
вместе с паникой LLVM выбросил. Поведение на кодах больше 7 другое (код 9
выполнится как 1), но определённое.

</div>

<div class="box check">

**Проверь себя.** Добавьте в `ch02_segments` ветку `10 => 0x77` (буква A).
Останется ли таблица значений? А если вместо неё добавить ветку
`200 => 0x77`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-armv7m]: ../appendix-f.md#armv7m "ARMv6-M, ARMv7-M, ARMv7E-M: Версии архитектуры ARM для микроконтроллеров."
[g-inlining]: ../appendix-f.md#inlining "Встраивание (inlining): Подстановка тела функции в место вызова."
[g-ir]: ../appendix-f.md#ir "Промежуточное представление (intermediate representation, IR): Форма программы между исходником и машинным кодом, удобная для анализа и оптимизации. rustc проходит через несколько: HIR, MIR, затем LLVM IR."
