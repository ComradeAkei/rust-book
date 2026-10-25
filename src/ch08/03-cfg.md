# 8.3. Условная компиляция

## #[cfg] и cfg!

Атрибут `#[cfg(условие)]` оставляет элемент в программе, только если
условие истинно. Его можно поставить на функцию, модуль, `impl`, поле
структуры, вариант перечисления, ветку `match` и отдельный оператор.
Условия складываются из имён (`test`, `debug_assertions`), пар
`имя = "значение"` (`target_arch = "arm"`, `feature = "std"`) и
[комбинаторов][g-combinator] `all(…)`, `any(…)` и `not(…)`.

Элемент с ложным условием удаляется до разрешения имён и проверки
типов. Он должен разобраться синтаксически, но может ссылаться на
несуществующие функции и типы, которых нет на этой цели: на x86_64
компилятор не смотрит, что внутри ветки для ARM. Поэтому вместо
`#ifdef` посреди функции в Rust пишут две функции с одной сигнатурой
под разными условиями.

Макрос `cfg!(условие)` — другое средство: он превращается в
`true` или `false`, а код вокруг остаётся на месте и проверяется
целиком. В ветке `if cfg!(…)` нельзя вызвать функцию, которой нет на
текущей цели, зато оптимизатор выбросит мёртвую ветку так же надёжно,
как препроцессор.

Атрибут `#[cfg_attr(условие, атрибут)]` навешивает атрибут по условию.
В этой главе он встречается дважды: корень `code/ch08/src/lib.rs`
объявляет `#![cfg_attr(not(any(test, feature = "std")), no_std)]`, то
есть библиотека без возможности `std` собирается без стандартной
библиотеки, а тесты помечены `#[cfg_attr(miri, ignore = "…")]`, потому
что под Miri нельзя запускать cargo.

## Признаки цели

Значения `target_*` задаёт выбранная цель. Их список печатает сам
компилятор:

```console
$ rustc --print cfg --target thumbv6m-none-eabi
debug_assertions
panic="abort"
target_abi="eabi"
target_arch="arm"
target_endian="little"
target_env=""
target_has_atomic_primitive_alignment="16"
target_has_atomic_primitive_alignment="32"
target_has_atomic_primitive_alignment="8"
target_has_atomic_primitive_alignment="ptr"
target_os="none"
target_pointer_width="32"
target_vendor="unknown"
$ diff <(rustc --print cfg --target thumbv6m-none-eabi) \
       <(rustc --print cfg --target thumbv7m-none-eabi)
6a7,10
> target_has_atomic="16"
> target_has_atomic="32"
> target_has_atomic="8"
> target_has_atomic="ptr"
```

Cortex-M0 и Cortex-M3 различаются в этом списке ровно одним признаком:
у [ARMv7-M][g-armv7m] есть атомарные операции чтения-изменения-записи, у ARMv6-M
нет. Сам тип `AtomicU32` на M0 есть, но только с `load` и `store`:
инструкций `LDREX`/`STREX` в ARMv6-M нет, и `fetch_add` выразить
нечем. Между M3 и M4F разница только в `target_abi` (`eabi` и
`eabihf`, передача `f32` в регистрах [FPU][g-fpu]).

<div class="box ref">

**Справка: эксклюзивный доступ в ARM.** `LDREX` читает слово и
запоминает в мониторе процессора, что адрес взят «под наблюдение».
`STREX` пишет слово, только если с тех пор монитор не сбросился: никто
другой не писал по этому адресу и не было исключения (вход в
обработчик и возврат из него сбрасывают монитор). Иначе запись не
выполняется, и `STREX` возвращает 1. Атомарное увеличение — это цикл
«прочитать, прибавить, попытаться записать», который повторяется, пока
`STREX` не вернёт 0. На одноядерном M0 атомарность получают иначе:
запрещают прерывания битом [PRIMASK][g-primask], и тогда между чтением и записью
никто не вклинится.

</div>

## Одна функция, две реализации

Счётчик событий, который увеличивают и основной код, и обработчики
прерываний, в `code/ch08/src/atomic.rs` выбирает реализацию по признаку
цели:

```rust
{{#include ../../code/ch08/src/atomic.rs:bump}}
```

Для каждой цели компилятор видит ровно одну `ch08_bump`. Вот что из неё
получилось:

```console
$ cargo asm -p ch08 --lib --simplify ch08_bump
ch08_bump:
	mov eax, 1
	lock xadd	dword ptr [rdi], eax
	ret
$ cargo asm -p ch08 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch08_bump
ch08_bump:
	push {r7, lr}
	mov r7, sp
	mov r1, r0
.LBB4_1:
	ldrex r0, [r1]
	adds r2, r0, #1
	strex r3, r2, [r1]
	cmp r3, #0
	bne .LBB4_1
	pop {r7, pc}
$ cargo asm -p ch08 --lib --simplify --no-default-features --target thumbv6m-none-eabi ch08_bump
ch08_bump:
	push {r7, lr}
	add r7, sp, #0
	mov r1, r0
	mrs r2, primask
	cpsid i
	ldr r0, [r0]
	adds r3, r0, #1
	str r3, [r1]
	lsls r1, r2, #31
	bne .LBB4_2
	cpsie i
.LBB4_2:
	pop {r7, pc}
```

На x86_64 весь `fetch_add` — одна инструкция с префиксом `lock`. На M3 —
цикл `ldrex`/`strex`, тот самый из справки. На M0 — наша вторая
реализация: сохранить PRIMASK, запретить прерывания, прочитать,
записать и разрешить их снова, только если они были разрешены до нас.
`lsls r1, r2, #31` сдвигает бит 0 сохранённого PRIMASK в старший разряд,
и флаг Z показывает, был ли он нулём. Прошивка
`firmware/ch08-sections` вызывает одну и ту же функцию на обоих
процессорах:

```rust
{{#include ../../firmware/ch08-sections/src/bin/target.rs:main}}
```

```console
$ cd firmware
$ cargo run --release -q -p ch08-sections --bin target
ch08_bump (LDREX/STREX): было 41, стало 42
…
$ cargo run --release -q -p ch08-sections --bin target --target thumbv6m-none-eabi
ch08_bump (запрет прерываний): было 41, стало 42
…
```

Третья ветка, `compile_error!`, срабатывает на цели, где нет ни
атомиков, ни способа запретить прерывания: сборка остановится с
понятным сообщением вместо молча неатомарного счётчика. Пока условие
ложно, `compile_error!` внутри `#[cfg]` никак себя не проявляет —
ещё одно следствие того, что такой элемент удаляется до проверки.

<div class="box trap">

**Ловушка.** Запрет прерываний на M0 атомарен только на одном ядре.
На многоядерном микроконтроллере (RP2040 — два ядра Cortex-M0+) второе
ядро продолжает работать, пока первое запретило у себя прерывания, и
нужна аппаратная блокировка. Поэтому в экосистеме критическую секцию
выбирают не по `target_arch`: крейт `critical-section` объявляет
интерфейс, а реализацию подставляет код поддержки конкретной платы
(глава 27).

</div>

## Опечатки

Повторим опыт из введения на Rust. Пакет `code/ch08/typo` объявляет
возможность `dma`, а код проверяет `dam`, и ещё ошибается в имени ОС:

```rust
{{#include ../../code/ch08/typo/src/lib.rs:typo}}
```

```console
$ cargo check
warning: unexpected `cfg` condition value: `dam`
 --> src/lib.rs:2:7
  |
2 | #[cfg(feature = "dam")]
  |       ^^^^^^^^^^-----
  |                 |
  |                 help: there is a expected value with a similar name: `"dma"`
  |
  = note: expected values for `feature` are: `dma`
  = help: consider adding `dam` as a feature in `Cargo.toml`
  …
warning: unexpected `cfg` condition value: `linxu`
 --> src/lib.rs:5:7
  |
5 | #[cfg(target_os = "linxu")]
  |       ^^^^^^^^^^^^-------
  |                   |
  |                   help: there is a expected value with a similar name: `"linux"`
  |
  = note: expected values for `target_os` are: `aix`, `amdhsa`, `android`, … and 14 more
```

Компилятор знает, какие значения допустимы. Для признаков цели список
встроен в rustc, а список возможностей ему передаёт Cargo из
`Cargo.toml`, это видно в `cargo check -v`:

```text
--check-cfg 'cfg(feature, values("dma"))'
```

Своё имя условия (например, `--cfg board_rev2` из сценария сборки)
нужно так же объявить, иначе будет то же предупреждение; как —
разобрано в разделе [8.5](05-build-rs.md). Тест
`code/ch08/tests/cargo.rs` собирает `typo` и проверяет оба
предупреждения.

<div class="box c">

**Как в C.** `#ifdef` работает с текстом до компилятора и не знает, какие
макросы вообще бывают: неопределённое имя — законное значение «нет».
`#[cfg]` работает с синтаксическим деревом, поэтому не может вырезать
половину выражения или незакрытую скобку, а имена условий проверяются
по известному списку. Цена — меньшая гибкость: нельзя
«условно определить» кусок текста посреди оператора, и для этого
пишут две функции.

</div>

<div class="box check">

**Проверь себя.** Почему в `report::line` из раздела
[8.4](04-features.md) использован `cfg!`, а не `#[cfg]`? Что
случится, если в ветке `if cfg!(target_arch = "arm")` вызвать функцию,
объявленную под `#[cfg(target_arch = "arm")]`, и собрать для x86_64?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-armv7m]: ../appendix-f.md#armv7m "ARMv6-M, ARMv7-M, ARMv7E-M: Версии архитектуры ARM для микроконтроллеров."
[g-combinator]: ../appendix-f.md#combinator "Комбинатор (combinator): Метод или функция, которая строит новое вычисление из готовых, не разбирая значение вручную: map, and_then, ok_or у Option и Result, адаптеры итераторов, all, any и not в условиях cfg."
[g-fpu]: ../appendix-f.md#fpu "FPU (floating-point unit): Блок плавающей точки в процессоре."
[g-primask]: ../appendix-f.md#primask "PRIMASK (priority mask): Однобитовый регистр маски Cortex-M: если бит установлен, ядро не принимает исключения с настраиваемым приоритетом, то есть все прерывания, кроме NMI и HardFault."
