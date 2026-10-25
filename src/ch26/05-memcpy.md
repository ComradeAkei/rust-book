# 26.5. memcpy без libc

## Вызовы, которых никто не писал

Ни одна функция ниже не вызывает `memcpy`, `memset`, `memcmp` или
`strlen`:

```rust
{{#include ../../code/ch26/src/copies.rs:copies}}
```

А в машинном коде вызывают — все, кроме двух на x86-64. Release-сборка
крейта главы, rustc 1.97:

| Функция | thumbv7m-none-eabi | thumbv6m-none-eabi | x86-64 Linux |
|---|---|---|---|
| `ch26_copy_frame` | `bl __aeabi_memcpy` | `bl __aeabi_memcpy` | `call memcpy` |
| `ch26_store` | `bl __aeabi_memcpy4` | `bl __aeabi_memcpy4` | `jmp memcpy` |
| `ch26_clear` | `bl __aeabi_memclr4` | `bl __aeabi_memclr4` | 16 × `movups` |
| `ch26_fill` | `bl __aeabi_memset` | `bl __aeabi_memset` | `jmp memset` |
| `ch26_same` | `bl memcmp` | `bl memcmp` | `pcmpeqb`, без вызова |
| `ch26_c_len` | `b strlen` | `bl strlen` | `jmp strlen` |

Тесты `tests/asm.rs` собирают крейт под три цели и сверяют вызовы в
каждой функции с этой таблицей. Компилятор вставляет вызовы по трём
причинам.

**Копия большого объекта.** `copy_from_slice` и присваивание `*slot =
*b` для 256 байт — это `memcpy`, и LLVM представляет их внутренней
операцией `llvm.memcpy`. Короткую копию известной длины он раскроет в
несколько загрузок и записей, длинную или неизвестной длины — сделает
вызовом. Обнуление массива — `llvm.memset`, сравнение массивов —
`memcmp`.

**Идиома в цикле.** `ch26_fill` — цикл, а не вызов, но проход LLVM
*LoopIdiomRecognize* узнаёт в нём `memset` и заменяет. Цикл в
`ch26_c_len` он узнаёт как `strlen`.

**Соглашения цели.** На ARM копию выполняет не `memcpy`, а
`__aeabi_memcpy`: так велит ARM EABI (документ RTABI, раздел 4.3.4).
Суффиксы `4` и `8` обещают выравнивание адресов: `Block` выровнен на 4,
и `ch26_store` зовёт `__aeabi_memcpy4`, которому можно копировать
словами без проверок.

```text
ch26_store:                       ch26_fill:
	push	{r7, lr}                  	cmp	r1, #0
	mov	r7, sp                    	it	eq
	mov.w	r2, #256                  	bxeq	lr
	bl	__aeabi_memcpy4           	push	{r7, lr}
	pop	{r7, pc}                  	mov	r7, sp
                                  	uxtb	r2, r2
                                  	bl	__aeabi_memset
                                  	pop	{r7, pc}
```

У `__aeabi_memset` порядок аргументов не тот, что у `memset`: адрес,
*длина*, значение. И `ch26_fill` передаёт срез без перестановок:
указатель в `r0`, длина в `r1` — как лежит толстый указатель, — а
значение в `r2`. Пустой срез отсеян до вызова.

На x86-64 обнуление 256 байт и сравнение 32 байт LLVM сделал векторами
SSE прямо на месте, а копию оставил `memcpy` из libc:

```text
ch26_store:
	mov	edx, 256
	jmp	qword ptr [rip + memcpy@GOTPCREL]
```

## Кто их даёт

На хосте — libc. Без libc на x86-64 Linux вызов `strlen` и стал
ошибкой компоновки в разделе [26.2](02-linux.md): `compiler_builtins`
для этой цели функций памяти не определяет, рассчитывая на libc.

На голом железе — `compiler_builtins`, крейт, который rustc подключает к
каждой программе сам. Его функции памяти — *слабые* символы:

```console
$ llvm-nm …/lib/rustlib/thumbv7m-none-eabi/lib/libcompiler_builtins-*.rlib \
    | grep -E ' (memcpy|__aeabi_memcpy|__aeabi_memclr4|memcmp|bcmp|strlen)$'
00000000 W __aeabi_memclr4
00000000 W __aeabi_memcpy
00000000 W bcmp
00000000 W memcmp
00000000 W memcpy
00000000 W strlen
```

`W` — слабое определение: если в программе есть обычное, сильное,
компоновщик возьмёт его, а слабое отбросит без ошибки о повторном
определении. Так программа может подставить свои функции — это
задача [26.D](08-tasks.md).

Прошивка `mem` из `ch26-bare` вызывает все шесть функций на Cortex-M3:

```rust,ignore
{{#include ../../firmware/ch26-bare/src/bin/mem.rs:main}}
```

```console
$ cargo run --release -p ch26-bare --bin mem
copy_from_slice: 40 байт, frame[39] = 117
присваивание 256 байт: сумма 2016, после обнуления 0
сравнение 32 байт: равны, после правки разные
strlen("станция") = 14
$ llvm-nm -S target/thumbv7m-none-eabi/release/mem | grep -E 'mem|aeabi|strlen'
00000af0 000002e8 t _RNvNtCs5b8OqQoTH8D_17compiler_builtins3mem6memcpy
00000a36 000000ae t __aeabi_memclr4
00000ae4 0000000c t __aeabi_memcpy
00000eb4 0000009c t __aeabi_memcpy4
00000dd8 000000dc t __aeabi_memset
0000098e 00000074 t memcmp
00000a02 00000034 t strlen
```

В готовой прошивке символы `compiler_builtins` — локальные (`t`):
`__aeabi_memcpy` на 12 байт — переход к общей `memcpy` на 744 байта,
которая копирует словами, когда может. `memmove` нет — его никто не
вызвал, и `--gc-sections` его выбросил. В строке «станция» 14 байт:
кириллица в UTF-8 — по два. Тест `symbols` в `run-demos.sh` проверяет,
что `__aeabi_memcpy` здесь — локальный символ из `compiler_builtins`.

## Своя memcpy, которая зовёт сама себя

Своя `memset` на Rust — это цикл. Но LLVM узнаёт в цикле `memset` и
вызывает её — то есть саму себя: бесконечная рекурсия и переполнение
стека при первом же вызове. Внутри функций с именами `memcpy`, `memset`
и `strlen` LLVM идиому не подставляет — он знает, что это они. Но
`__aeabi_memset`, `__aeabi_memclr` или `my_memset` с тем же циклом
получат вызов `memset` или `__aeabi_memset` — и если это и есть вы,
рекурсию.

Для этого у крейта есть атрибут `#![no_builtins]`: в нём LLVM не
заменяет циклы вызовами функций памяти. Им помечены `compiler_builtins`
и решение задачи D. Тест `solutions/ch26/tests/naive.rs` собирает
решение без атрибута и находит в каждой обёртке `__aeabi_*` вызов
самой себя; при `opt-level = 0` рекурсии нет, оптимизаций нет.

<div class="box c">

**Как в C.** GCC 13 узнаёт идиому даже там, где LLVM останавливается:
`memset`, написанная циклом и названная `memset`, при `-O2` вызывает
саму себя — arm-none-eabi-gcc 13.2 ставит `bl memset` в её же тело.
`-nostdlib` не помогает: это ключ компоновки, а не компиляции.
Помогают `-ffreestanding` или `-fno-builtin` — с ними цикл остаётся
циклом. `code/ch26/c/run.sh` проверяет все четыре случая. При этом
руководство GCC требует от автономной среды `memcpy`, `memmove`,
`memset` и `memcmp` даже с `-ffreestanding`: копии структур компилятор
делает их вызовами, как и LLVM. В Rust роль `-fno-builtin` для крейта
играет `#![no_builtins]`.

</div>

<div class="box trap">

**Ловушка.** «Свой» `memcpy` на хосте опасен иначе. Экспортированный
под именем `memcpy` в тестовой программе, он подменил бы `memcpy` из
glibc для всего процесса — включая сам тестовый фреймворк и
распределитель памяти. Поэтому решение задачи D экспортирует свои
функции только при `target_os = "none"`, а на хосте это обычные функции
Rust с искажёнными именами.

</div>

<div class="box check">

**Проверь себя.** `sh::put` в `ch26-bare` начинается с `let mut buf =
[0u8; 64]`. Какой вызов функции памяти появится в её машинном коде на
Cortex-M3 и с какими аргументами?

</div>
