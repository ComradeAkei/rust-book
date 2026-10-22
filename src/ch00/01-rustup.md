# 0.1. rustup: тулчейны и цели

## Тулчейн

*Тулчейн* — это согласованный набор из `rustc`, `cargo`, стандартной
библиотеки и вспомогательных программ (`rustfmt`, `clippy`, Miri). Их ставит
и переключает `rustup`. Каналов три: stable выходит раз в шесть недель,
beta — будущий stable, nightly собирается каждую ночь и единственный
разрешает нестабильные возможности (`#![feature(...)]` и ключи `-Z`).

Книга закрепляет версию файлом в корне репозитория:

```toml
{{#include ../../rust-toolchain.toml}}
```

Достаточно зайти в каталог, и rustup сам поставит нужную версию с
компонентами и целями:

```console
$ rustup show active-toolchain
1.97.0-x86_64-unknown-linux-gnu (overridden by '.../rust-book/rust-toolchain.toml')
$ rustc -vV | grep -E 'release|LLVM'
release: 1.97.0
LLVM version: 22.1.6
```

Nightly в книге нужен только для инструментов: Miri, санитайзеров и
диагностических ключей вроде `-Zprint-type-sizes`. Его вызывают явно,
знаком `+` перед именем канала: `cargo +nightly miri test`. Код глав и
решения задач собираются на stable.

## Цели

*Цель* (target) описывает, для чего генерируется код: архитектуру,
операционную систему и ABI. Имя цели называют триплетом, хотя частей в нём
обычно четыре:

```text
x86_64 - unknown - linux - gnu         архитектура, производитель, ОС, окружение (libc)
thumbv7em - none - eabihf              набор инструкций, ОС нет, ABI
```

Для голого железа ОС и производителя нет, а первая часть называет не
процессор, а архитектуру набора команд. Цели Cortex-M в книге:

| Цель | Ядра | Архитектура | Чего нет по сравнению со следующей строкой |
|---|---|---|---|
| `thumbv6m-none-eabi` | Cortex-M0, M0+ | ARMv6-M | деления, `clz`, атомарных чтения-изменения-записи, невыровненного доступа |
| `thumbv7m-none-eabi` | Cortex-M3 | ARMv7-M | DSP-инструкций и FPU |
| `thumbv7em-none-eabihf` | Cortex-M4F, M7F | ARMv7E-M с FPU | — |

Разницу видно без единой строки кода: rustc печатает условия компиляции
`cfg`, которые определены для цели.

```console
$ rustc --print cfg --target thumbv6m-none-eabi | grep -E 'atomic|pointer'
target_has_atomic_primitive_alignment="16"
target_has_atomic_primitive_alignment="32"
target_has_atomic_primitive_alignment="8"
target_has_atomic_primitive_alignment="ptr"
target_pointer_width="32"
$ rustc --print cfg --target thumbv7m-none-eabi | grep -E 'atomic=|pointer'
target_has_atomic="16"
target_has_atomic="32"
target_has_atomic="8"
target_has_atomic="ptr"
target_pointer_width="32"
```

У ARMv6-M нет `target_has_atomic`: в наборе команд нет пары
`LDREX`/`STREX`, а значит, нет атомарного `fetch_add` и `compare_exchange`.
Атомарные загрузка и сохранение есть (выровненное слово читается одной
инструкцией), но всё, что сложнее, на Cortex-M0 делают через запрет
прерываний. Прошивки главы 6 уже пользовались этим признаком, чтобы
отличить M0 от M3 при компиляции. Подробно об атомиках — в главе 21.

Полное описание цели печатает nightly:

```console
$ rustc +nightly -Z unstable-options --print target-spec-json --target thumbv7em-none-eabihf \
>   | grep -E '"(abi|features|frame-pointer|linker|llvm-floatabi|max-atomic-width|panic-strategy|relocation-model)"'
  "abi": "eabihf",
  "features": "+vfp4d16sp",
  "frame-pointer": "always",
  "linker": "rust-lld",
  "llvm-floatabi": "hard",
  "max-atomic-width": 32,
  "panic-strategy": "abort",
  "relocation-model": "static",
```

Каждая строка здесь — решение, которое в C принимается флагами
компилятора: `+vfp4d16sp` — FPU одинарной точности с 16 регистрами двойной
ширины (это `-mfpu=fpv4-sp-d16`), `llvm-floatabi: hard` — числа с
плавающей точкой передаются в регистрах FPU (`-mfloat-abi=hard`),
`relocation-model: static` — код не перемещаемый, адреса известны при
компоновке, `panic-strategy: abort` — раскрутки стека нет,
`max-atomic-width: 32` — атомарные операции не шире слова,
`frame-pointer: always` — указатель кадра в `r7` в каждой функции (отсюда
пролог `push {r7, lr}; mov r7, sp`, который глава 6 видела во всех
листингах для Cortex-M). Компоновщик `rust-lld` идёт вместе с rustc,
отдельный `arm-none-eabi-ld` не нужен.

<div class="box ref">

**Справка: что значит hf в eabihf.** EABI (Embedded ABI) — соглашение ARM
о том, как функции передают аргументы, раскладывают структуры и
пользуются стеком (подробно в разделе [0.4](04-abi.md)). У него два
варианта для чисел с плавающей точкой. В *soft-float* `f32` передаётся в
обычных регистрах `r0`–`r3`, как целое, и функция обязана уметь работать
без FPU. В *hard-float* (суффикс `hf`) — в регистрах FPU `s0`–`s15`. Код,
собранный по разным вариантам, нельзя смешивать в одной программе:
вызывающая сторона положит аргумент не туда, где его ищет вызываемая.

</div>

<div class="box trap">

**Ловушка.** На Cortex-M4F можно собрать прошивку и для
`thumbv7em-none-eabi`, без `hf`. Она заработает, но FPU будет простаивать:
у этой цели в описании нет ни одной возможности FPU, и умножение `f32`
превращается в вызовы программной реализации из `compiler_builtins`.

```text
thumbv7em-none-eabi:            thumbv7em-none-eabihf:
ch00_scale:                     ch00_scale:
	push {r7, lr}                   push {r7, lr}
	mov r7, sp                      mov r7, sp
	bl __aeabi_fmul                 vmov.f32 s2, #1.000000e+00
	mov.w r1, #1065353216           vmla.f32 s2, s0, s1
	bl __aeabi_fadd                 vmov.f32 s0, s2
	pop {r7, pc}                    pop {r7, pc}
```

Это функция `x * k + 1.0` из раздела [0.4](04-abi.md). Число 1065353216
слева — это 0x3F80_0000, битовое представление `1.0f32`. Включить FPU,
сохранив soft-float ABI, можно ключом
`-C target-feature=+vfp4d16sp`, но rustc предупредит, что этот ключ
нестабилен для ARM (`unknown and unstable feature specified`). Выбирайте
цель по ядру, а не по наитию.

</div>

<div class="box c">

**Как в C.** В GNU-тулчейне для ARM все варианты собраны в один
компилятор, а нужную `libc` и `libgcc` он выбирает по флагам из набора
*multilib*:

```console
$ arm-none-eabi-gcc -print-multi-lib | grep v7e-m
thumb/v7e-m/nofp;@mthumb@march=armv7e-m@mfloat-abi=soft
thumb/v7e-m+fp/softfp;@mthumb@march=armv7e-m+fp@mfloat-abi=softfp
thumb/v7e-m+fp/hard;@mthumb@march=armv7e-m+fp@mfloat-abi=hard
...
```

Ошибка компоновки из мотивирующего примера пришла из
`thumb/v7e-m/nofp/libc.a`: флага `-mfloat-abi=hard` в той команде не было,
и GCC выбрал библиотеку без FPU. В Rust этот выбор сделан один раз в имени
цели и не может разойтись между объектными файлами одной сборки.

</div>

## Установка целей

```console
$ rustup target add thumbv6m-none-eabi thumbv7m-none-eabi thumbv7em-none-eabihf
$ rustup target list --installed
```

В репозитории книги это делать не нужно: цели перечислены в
`rust-toolchain.toml`. Компонент цели — это скомпилированные `core`,
`alloc` и `compiler_builtins`. Последний содержит те самые `__aeabi_fmul`
и `__aeabi_lmul`, которые компилятор вызывает, когда у ядра нет нужной
инструкции. Стандартной библиотеки `std` для `none` нет: в ней нечему
реализовывать файлы, потоки и сеть.

<div class="box check">

**Проверь себя.**

1. Почему у цели для Cortex-M0 нет `target_has_atomic="32"`, хотя
   32-битное слово читается одной инструкцией?
2. Что сломается, если половину прошивки собрать для
   `thumbv7em-none-eabihf`, а библиотеку на C к ней — с
   `-mfloat-abi=soft`?
3. Какие строки описания цели соответствуют флагам GCC `-mfpu` и
   `-mfloat-abi`?

</div>
