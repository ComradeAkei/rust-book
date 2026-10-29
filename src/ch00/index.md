# Глава 0. Окружение и инструменты

Перед началом достаточно понимать функцию, переменную и цикл.
Если слова «регистр», «целевая платформа» и «компоновщик» пока незнакомы,
прочитайте [основы](../foundations.md). Задача главы — научиться собрать
и проверить небольшой пример. Имена всех инструментов и инструкции
процессора запоминать сразу не нужно: сначала повторите один опыт
и объясните, что именно он проверяет.

## Мотивирующая проблема

Вы хотите узнать, во что компилируется сумма массива на x86_64 и на
Cortex-M4F. На C это выглядит так:

```c
#include <stddef.h>
#include <stdint.h>

uint32_t sum(const uint32_t *xs, size_t n)
{
    uint32_t s = 0;
    for (size_t i = 0; i < n; i++)
        s += xs[i];
    return s;
}

int main(void)
{
    static const uint32_t samples[] = { 1, 2, 3 };
    return (int)sum(samples, 3);
}
```

Собираем с оптимизацией и смотрим на `main`:

```console
$ gcc -O2 -S -masm=intel -o - sum.c          # GCC 13.3, x86_64
main:
	endbr64
	mov	eax, 6
	ret

$ arm-none-eabi-gcc -O2 -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 -c sum.c
$ arm-none-eabi-objdump -d sum.o
00000000 <main>:
   0:	2006      	movs	r0, #6
   2:	4770      	bx	lr
```

Вызова нет: компилятор сложил три числа сам и вернул 6. Тело `sum` в
объектном файле осталось, но то ли это тело, которое выполнится в
настоящей программе, где массив приходит из АЦП, а не из константы, —
вопрос открытый. Это первая трудность: оптимизатор отвечает не на тот
вопрос, который вы задали.

Вторая трудность — сама сборка под микроконтроллер. Для Cortex-M нужен
отдельный компилятор `arm-none-eabi-gcc`, четыре флага, которые описывают
ядро и способ передачи чисел с плавающей точкой, а к ним стартовый файл с
[таблицей векторов][g-vector-table] и *скрипт компоновщика* (linker script), который
раскладывает секции программы по адресам флеш-памяти и ОЗУ. Без них
компоновка падает:

```text
$ arm-none-eabi-gcc -O2 -mcpu=cortex-m4 -mthumb sum.c -o sum.elf
ld: .../thumb/v7e-m/nofp/libc.a(libc_a-exit.o): in function `exit':
exit.c:65:(.text+0x14): undefined reference to `_exit'
ld: .../thumb/v7e-m/nofp/libc.a(libc_a-writer.o): in function `_write_r':
writer.c:49:(.text+0x14): undefined reference to `_write'
...
```

А когда прошивка наконец собрана, её нужно где-то запустить и посмотреть,
что происходит до `main`.

## Что делает Rust

- Один компилятор знает все [цели][g-target]. `rustup target add thumbv7em-none-eabihf`
  ставит готовую [`core`][g-core-alloc-std] для Cortex-M4F, а ключ `--target` у cargo заменяет
  набор флагов `-mcpu`, `-mfloat-abi`, `-mfpu`: всё это записано в описании
  цели.
- *Стартовый код* (startup code) — всё, что выполняется от сброса до
  `main`, — и скрипт компоновщика для Cortex-M дают [крейты][g-crate] экосистемы
  (`cortex-m-rt`), и подключаются они так же, как любая зависимость.
- `cargo asm`, Miri и [clippy][g-clippy] ставятся одной командой и работают с тем же
  [workspace][g-workspace], что и сборка.

Эта глава не про язык, а про инструменты, которыми проверяется каждое
утверждение книги. К её концу вы соберёте один исходник под x86_64 и три
ядра Cortex-M, прочитаете его ассемблер и пройдёте отладчиком путь
прошивки от сброса до `main`.

## План главы

| Раздел | О чём |
|---|---|
| [0.1](01-rustup.md) | rustup: тулчейны, цели, что такое `thumbv7em-none-eabihf` |
| [0.2](02-cargo.md) | cargo: workspace книги, профили, проверки переполнения |
| [0.3](03-asm.md) | Как читать результат компиляции: `cargo asm`, LLVM IR, `nm`, `llvm-objdump` |
| [0.4](04-abi.md) | Один исходник, две ISA: соглашения о вызовах System V AMD64 и AAPCS |
| [0.5](05-checks.md) | Проверка корректности: Miri, санитайзеры, clippy |
| [0.6](06-qemu.md) | Железо без платы: QEMU, gdb, probe-rs и defmt |
| [0.7](07-tasks.md) | Задачи A–D |

## Код главы

```console
$ cargo test -p ch00 -p ch00-solutions        # примеры и решения
$ cargo asm -p ch00 --lib ch00_args8          # ассемблер функции из раздела 0.4
$ cargo +nightly miri run -p ch00 --example ub
$ cd firmware && bash run-demos.sh            # прошивки главы на Cortex-M0, M3 и M4F
```

Числа в тексте получены на rustc 1.97.0 ([LLVM][g-llvm] 22.1.6) для
`x86_64-unknown-linux-gnu`, `thumbv6m-none-eabi`, `thumbv7m-none-eabi` и
`thumbv7em-none-eabihf`, QEMU 8.2.2 и gdb 15.1. Ассемблер снят командой
`cargo asm --simplify` из release-сборки, если не сказано иное.

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-clippy]: ../appendix-f.md#clippy "clippy: Линтер из тулчейна Rust: несколько сотен линтов, от неидиоматичного кода до заведомых ошибок, разбитых на группы с уровнями allow, warn и deny."
[g-core-alloc-std]: ../appendix-f.md#core-alloc-std "core, alloc, std: Три слоя стандартной библиотеки. core ничего не требует от среды; alloc добавляет Box, Vec, String и требует распределитель памяти; std переэкспортирует оба и добавляет то, что требует ОС: потоки, файлы, сеть."
[g-crate]: ../appendix-f.md#crate "Крейт (crate): Единица компиляции Rust: одно дерево модулей, один вызов rustc, один результат — библиотека или программа."
[g-llvm]: ../appendix-f.md#llvm "LLVM: Компиляторная инфраструктура, на которой построены rustc и clang: принимает LLVM IR, оптимизирует его и генерирует машинный код под цель."
[g-target]: ../appendix-f.md#target "Цель (target): То, для чего генерируется код: архитектура, ОС и ABI, записанные в имени-триплете (thumbv7m-none-eabi)."
[g-vector-table]: ../appendix-f.md#vector-table "Таблица векторов (vector table): Массив слов в начале образа Cortex-M: слово 0 — начальное значение указателя стека, слово 1 — адрес обработчика сброса, дальше адреса обработчиков исключений и прерываний."
[g-workspace]: ../appendix-f.md#workspace "Рабочее пространство (workspace): Несколько пакетов cargo с общим Cargo.lock, общим каталогом target/ и общими профилями."
