# 0.3. Как читать результат компиляции

## cargo asm

Главный инструмент книги — `cargo asm` из пакета `cargo-show-asm`
(`cargo install cargo-show-asm`). Он собирает крейт так же, как
`cargo build`, просит rustc сохранить ассемблер и показывает одну функцию.
Без имени функции он печатает список того, что нашёл:

```console
$ cargo asm -p ch00 --lib
0 "ch00_add" [9]
1 "ch00_args8" [49]
2 "ch00_average_bad" [27]
3 "ch00_keep" [42]
4 "ch00_mul_wide" [12]
5 "ch00_pair" [10]
6 "ch00_quad" [16]
7 "ch00_scale" [10]
```

В квадратных скобках число строк листинга. Функцию выбирают по имени или
по номеру. Имя работает как фильтр по подстроке: если ему подходят
несколько функций, `cargo asm` снова печатает список, и тогда после имени
добавляют номер из этого списка. Самая простая функция главы:

```rust
{{#include ../../code/ch00/src/hood.rs:add}}
```

```console
$ cargo asm -p ch00 --lib ch00_add
.section .text.ch00_add,"ax",@progbits
	.globl	ch00_add
	.p2align	4
.type	ch00_add,@function
ch00_add:
	.cfi_startproc
	lea eax, [rdi + rsi]
	ret
```

Функции `ch00_add_wrapping` из того же файла в списке нет. Её тело в
release совпало с телом `ch00_add` до инструкции, и LLVM оставил одну
копию, а второе имя сделал псевдонимом: в полном листинге
(`--everything`) стоит строка `ch00_add_wrapping = ch00_add`. Подробнее
об этом слиянии — в разделе [2.2](../ch02/02-loops.md).

Строки с точкой — директивы ассемблеру, а не инструкции: в какую секцию
положить функцию, как её выровнять, что записать в таблицу раскрутки
стека (`.cfi_*`). Ключ `--simplify` их убирает, и дальше в книге листинги
приводятся в таком виде. Остаются две инструкции: `lea` вычисляет адрес
`rdi + rsi`, но адрес никто не читает, это просто сложение двух регистров
с записью в третий без изменения флагов. Аргументы пришли в `rdi` и `rsi`,
результат ушёл в `eax`: так требует соглашение о вызовах, о нём раздел
[0.4](04-abi.md).

Остальные ключи, которые понадобятся:

| Ключ | Что показывает |
|---|---|
| `--target thumbv7m-none-eabi` | Код для другой цели |
| `--dev` | Профиль dev вместо release |
| `--llvm` | LLVM IR вместо ассемблера |
| `--mir` | MIR, промежуточное представление rustc |
| `--example fold`, `--bin имя` | Функции примера или программы, а не библиотеки |

## Debug-сборка лжёт о стоимости

Та же функция в профиле dev. Без оптимизаций функции не сливаются, и
имени `ch00_add` подходят две: `ch00_add` и `ch00_add_wrapping`. Номер 0
после имени выбирает первую:

```console
$ cargo asm -p ch00 --lib --dev --simplify ch00_add 0
ch00_add:
	sub rsp, 24
	mov eax, edi
	mov dword ptr [rsp + 16], eax
	mov dword ptr [rsp + 20], esi
	mov edi, eax
	add edi, esi
	mov dword ptr [rsp + 12], edi
	cmp edi, eax
	jb .LBB10_2
	mov eax, dword ptr [rsp + 12]
	add rsp, 24
	ret
.LBB10_2:
	lea rdi, [rip + .Lanon.bf21702488e6121fb311b8aa10c5ed51.5]
	call qword ptr [rip + core::panicking::panic_const::panic_const_add_overflow@GOTPCREL]
```

Для Cortex-M3 (`--target thumbv7m-none-eabi`) картина та же:

```text
ch00_add:
	push {r7, lr}
	mov r7, sp
	sub sp, #16
	str r1, [sp]
	mov r1, r0
	ldr r0, [sp]
	str r1, [sp, #8]
	str r0, [sp, #12]
	add r0, r1
	mov r2, r0
	str r2, [sp, #4]
	cmp r0, r1
	blo .LBB10_2
	b .LBB10_1
.LBB10_1:
	ldr r0, [sp, #4]
	add sp, #16
	pop {r7, pc}
.LBB10_2:
	movw r0, :lower16:.Lanon.5e89357b0e826a5923765716217ad3ae.5
	movt r0, :upper16:.Lanon.5e89357b0e826a5923765716217ad3ae.5
	bl core::panicking::panic_const::panic_const_add_overflow
```

В release на Cortex-M3 от неё остаются `add r0, r1` и пролог с эпилогом.

На x86_64 двенадцать инструкций на пути без переполнения вместо двух, и
почти все они не про сложение.
Аргументы сохраняются на стек, чтобы отладчик мог показать их значения в
любой момент. Результат сохраняется, а потом снова загружается. Проверка
переполнения устроена так: беззнаковая сумма переполнилась тогда и только
тогда, когда она меньше слагаемого (`cmp edi, eax; jb`). Ветка паники
передаёт в `rdi` адрес структуры с именем файла и номером строки.

<div class="box trap">

**Ловушка.** Ассемблер debug-сборки почти ничего не говорит о реальной
стоимости кода. Обращения к стеку, лишние переходы и копии в нём — следы
`opt-level = 0`, а не свойства языка. Рассуждения «итераторы медленнее
цикла», построенные на debug-листинге, неверны. Все листинги книги сняты
из release, если не сказано обратное.

</div>

## LLVM IR

rustc переводит программу в промежуточное представление LLVM, а уже LLVM
выбирает инструкции под цель. IR одинаков для x86_64 и ARM, поэтому по нему
удобно понимать, что обещал компилятору Rust, а что придумал бэкенд.

```console
$ cargo asm -p ch00 --lib --llvm ch00_add --dev 0
define i32 @ch00_add(i32 %a, i32 %b) unnamed_addr #3 !dbg !477 {
start:
  %b.dbg.spill = alloca [4 x i8], align 4
  %a.dbg.spill = alloca [4 x i8], align 4
  ...
  %_3.0 = add i32 %a, %b, !dbg !486
  %_3.1 = icmp ult i32 %_3.0, %a, !dbg !486
  br i1 %_3.1, label %panic, label %bb1, !dbg !486

$ cargo asm -p ch00 --lib --llvm ch00_add
define noundef i32 @ch00_add(i32 noundef %a, i32 noundef %b) unnamed_addr #0 !dbg !7 {
start:
  %_0 = add i32 %b, %a, !dbg !19
  ret i32 %_0, !dbg !20
}
```

В release осталась одна инструкция `add`, и у неё нет флагов `nuw` или
`nsw` (no unsigned wrap, no signed wrap). Их отсутствие означает «при
переполнении результат по модулю 2³²». Clang для `int a + b` ставит `nsw`:
так знаковое переполнение в C превращается в обещание оптимизатору, что
переполнения не бывает. Атрибуты `noundef` говорят, что аргументы и
результат — инициализированные значения; это обещание Rust, а не C.

Без `cargo asm` IR получают так:

```console
$ cargo rustc -p ch00 --lib --release -- --emit=llvm-ir
$ ls target/release/deps/ch00-*.ll
```

## Функция, которая исчезла

Пример `fold` вызывает две одинаковые функции:

```rust
{{#include ../../code/ch00/examples/fold.rs:fold}}
```

```console
$ cargo asm -p ch00 --example fold
0 "<std::rt::lang_start<()>::{closure#0} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}" [20]
1 "fold::main" [42]
2 "fold::square_kept" [12]
3 "main" [18]
...
$ cargo asm -p ch00 --example fold fold::main
fold::main:
	sub rsp, 40
	mov dword ptr [rsp], 144
	mov dword ptr [rsp + 8], 12
	lea rax, [rsp + 8]
	#APP
	#NO_APP
	mov edi, dword ptr [rsp + 8]
	call fold::square_kept
	mov dword ptr [rsp + 4], eax
	...
	call qword ptr [rip + std::io::stdio::_print@GOTPCREL]
```

Функции `square` в списке нет. Её вызов встроен в `main`, а результат
посчитан при компиляции: `mov dword ptr [rsp], 144` кладёт готовое 12² на
стек для печати. Функция `square_kept` осталась отдельной, и вызов идёт по
всем правилам: аргумент в `edi`, результат в `eax`.

Два механизма удержали её:

- `#[inline(never)]` запрещает встраивать тело в место вызова;
- `std::hint::black_box(12)` прячет от оптимизатора значение аргумента. В
  листинге он виден как пустая вставка ассемблера между `#APP` и
  `#NO_APP`: компилятор обязан положить 12 в память, отдать её адрес
  «неизвестному коду» и прочитать значение обратно, не зная, что код
  пустой.

Ещё один атрибут — `#[unsafe(no_mangle)]` — сохраняет имя функции как
есть. Функции книги, которые разбираются в ассемблере, помечены
`no_mangle`, чтобы имя было одинаковым под любой целью, и
`inline(never)`, чтобы тело существовало отдельно, а там, где их
вызывают, аргументы проходят через `black_box`, чтобы не сработала
свёртка констант. Обёртка `unsafe(...)` вокруг
`no_mangle` обязательна с edition 2024 (сам синтаксис появился в Rust 1.82
и годится в любой редакции): два крейта с одинаковым неискажённым именем
дадут конфликт символов при компоновке, и программист берёт эту
ответственность на себя явно.

<div class="box trap">

**Ловушка.** Ни один из трёх способов по отдельности не гарантирует, что
функция окажется в готовом бинарнике. Сколько способов у оптимизатора и
компоновщика её убрать, вы выясните в задаче 0.A. Начните с вопроса: если
`#[inline(never)]`-функцию вызывают с константой, обязан ли компилятор её
вызывать?

</div>

## nm, objdump и манглинг

`cargo asm` показывает объектный код одного крейта. Что попало в готовый
исполняемый файл, говорят `nm` (таблица символов) и `objdump` или
`llvm-objdump` (дизассемблер):

```console
$ nm target/release/examples/fold | grep 4fold
0000000000013ce0 t _RNvCs4p50bBigtaS_4fold11square_kept
0000000000013cf0 t _RNvCs4p50bBigtaS_4fold4main
...
$ nm -C target/release/examples/fold | grep fold::
0000000000013ce0 t fold::square_kept
0000000000013cf0 t fold::main
$ objdump -d -C -M intel --no-show-raw-insn target/release/examples/fold | grep -A4 '<fold::square_kept>:'
0000000000013ce0 <fold::square_kept>:
   13ce0:	mov    eax,edi
   13ce2:	imul   eax,edi
   13ce5:	ret
   13ce6:	int3
```

Имена в таблице символов искажены (mangled): в них закодирован путь
`fold::square_kept`, а `Cs4p50bBigtaS_` — хеш крейта, чтобы одинаковые
имена из разных крейтов и версий не столкнулись. rustc 1.97 использует
схему v0, её имена начинаются с `_R`. Старая схема давала имена вида
`_ZN4fold11square_kept17h<хеш>E`, похожие на C++. Ключ `-C` у `nm` и
`objdump` из binutils 2.42 переводит обе схемы обратно в пути Rust. Буква
`t` означает локальный символ в секции кода, `T` — глобальный. Байты
`int3` после `ret` — заполнение до выравнивания следующей функции на 16:
если управление туда всё-таки попадёт, процессор сразу остановится на
точке останова.

Для прошивок нужен дизассемблер, который знает Thumb: `llvm-objdump` из
пакета LLVM или `arm-none-eabi-objdump`.

## Что занимает место: cargo-bloat

```console
$ cargo bloat --release -p ch00 --example hello --crates
File  .text     Size Crate
5.7%  98.6% 241.6KiB std
0.0%   0.0%      84B [Unknown]
0.0%   0.0%      21B __rustc
0.0%   0.0%      18B hello
5.8% 100.0% 245.0KiB .text section size, the file size is 4.2MiB
```

Из 245 КиБ кода hello world на долю самой программы приходится 18 байт,
а сам файл весит 4,2 МиБ, хотя код занимает 5,8 % от него. Что такое
остальные 241 КиБ кода и 4 МиБ сверх них и как от них избавиться — задача
0.B.
Без ключа `--crates` cargo-bloat перечисляет крупнейшие функции.

<div class="box c">

**Как в C.** Соответствие инструментов:

| C | Rust |
|---|---|
| `gcc -S -fverbose-asm` | `cargo asm` |
| `clang -S -emit-llvm` | `cargo asm --llvm`, `--emit=llvm-ir` |
| `objdump -d`, `nm`, `size` | те же, плюс `-C` для имён Rust |
| `bloaty` | `cargo bloat` (и тот же `bloaty`) |
| `__attribute__((noinline))` | `#[inline(never)]` |
| `asm volatile("" : : "r"(x) : "memory")` | `std::hint::black_box(x)` |

Compiler Explorer (godbolt.org) понимает Rust так же, как C: выберите
компилятор rustc и добавьте флаги `-C opt-level=3` и, для Cortex-M,
`--target thumbv7em-none-eabihf`. Для коротких опытов это быстрее
локальной сборки.

</div>

<div class="box check">

**Проверь себя.**

1. Почему debug-сборка `ch00_add` сохраняет аргументы на стек, хотя они
   уже лежат в регистрах?
2. Что изменится в IR release-сборки `ch00_add`, если включить
   `overflow-checks`?
3. Чем `#[inline(never)]` отличается от `black_box` по тому, что именно
   они скрывают от оптимизатора?

</div>
