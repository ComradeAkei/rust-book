# 0.6. Железо без платы: QEMU, gdb, probe-rs и defmt

Плата для книги не нужна. `qemu-system-arm` эмулирует микроконтроллер
целиком: ядро, флеш-память, ОЗУ и часть периферии. Инструкции он
выполняет точно по архитектуре, вплоть до исключений при невыровненном
доступе, которые разбирала глава 6. Чего QEMU не моделирует, так это
время: он переводит блоки команд ARM в машинный код хоста, у него нет
конвейера, кэшей и тактов ожидания флеш-памяти. Время выполнения в QEMU
ничего не говорит о времени на плате.

## Машины

| Машина QEMU | Ядро | Кристалл | Флеш / ОЗУ | Цель Rust |
|---|---|---|---|---|
| `microbit` | Cortex-M0 | Nordic nRF51822 | 256 КиБ / 16 КиБ | `thumbv6m-none-eabi` |
| `lm3s6965evb` | Cortex-M3 | TI Stellaris LM3S6965 | 256 КиБ / 64 КиБ | `thumbv7m-none-eabi` |
| `mps2-an385` | Cortex-M3 | плата ARM MPS2, образ FPGA AN385 | 4 МиБ / 4 МиБ SSRAM | `thumbv7m-none-eabi` |
| `mps2-an386` | Cortex-M4F | плата ARM MPS2, образ FPGA AN386 | 4 МиБ / 4 МиБ SSRAM | `thumbv7em-none-eabihf` |

Книга запускает Cortex-M0 на `microbit`, Cortex-M3 на `lm3s6965evb`
(его UART разбирался в главе 6) и Cortex-M4F на `mps2-an386`. Машина
`mps2-an385` — запасной вариант для M3 с той же картой памяти, что у
AN386. Размеры памяти сняты командой монитора QEMU `info mtree`. У всех
четырёх машин код начинается с адреса 0, а ОЗУ — с `0x2000_0000`, поэтому
прошивкам книги хватает одного файла `firmware/memory.x` с наименьшими
размерами.

<div class="box ref">

**Справка: карта памяти Cortex-M.** Архитектура ARMv6-M и ARMv7-M делит
4 ГиБ адресов на области с заранее заданным назначением. Код (обычно
флеш) — с `0x0000_0000`, ОЗУ — с `0x2000_0000`, периферия кристалла — с
`0x4000_0000`, системные регистры ядра — с `0xE000_0000`. В последней
области лежат блок управления системой (SCB), контроллер прерываний NVIC
и таймер SysTick, и их адреса одинаковы на любом Cortex-M. Поэтому
регистр CPUID по адресу `0xE000_ED00` читается одной и той же строкой
кода на всех трёх ядрах, а UART у каждого кристалла свой: `0x4000_C000`
у LM3S6965, `0x4000_2000` у nRF51822, `0x4000_4000` у MPS2.

С нулевого адреса начинается *таблица векторов*. Слово 0 — начальное
значение указателя стека, слово 1 — адрес обработчика сброса `Reset`,
дальше адреса обработчиков исключений и прерываний. После сброса ядро
само загружает SP и PC из первых двух слов и начинает выполнять `Reset`.
Младший бит адреса обработчика равен 1: так ARM помечает код Thumb, а
Cortex-M другого кода не выполняет.

</div>

## Раннер cargo

Как запускать прошивку, cargo узнаёт из `firmware/.cargo/config.toml`:

```toml
{{#include ../../firmware/.cargo/config.toml}}
```

`cargo run` собирает прошивку и вызывает `runner`, дописав в конец путь к
ELF-файлу. Ключ `-kernel` загружает ELF в память машины по адресам из его
программных заголовков, `-nographic` отправляет последовательный порт и
монитор QEMU в терминал. Ключ компоновщика `-Tlink.x` подключает скрипт
из крейта `cortex-m-rt`. Тот описывает таблицу векторов и секции, а
размеры памяти берёт из `memory.x`, который `build.rs` каждой прошивки
кладёт рядом.

<div class="box ref">

**Справка: semihosting.** Пока UART не настроен, прошивка печатает через
*semihosting*: просит отладчик выполнить ввод-вывод за неё. Номер
операции кладётся в `r0`, указатель на параметры — в `r1`, а затем
выполняется точка останова с особым номером:

```text
$ llvm-objdump -d .../thumbv6m-none-eabi/release/ch00-hello | grep -B2 bkpt
     214: 2018         	movs	r0, #0x18
     216: 4914         	ldr	r1, [pc, #0x50]
     218: beab         	bkpt	#0xab
```

`0x18` — операция `SYS_EXIT`, ею `debug::exit` завершает QEMU. Отладчик,
поймав `BKPT 0xAB`, выполняет операцию на хосте и продолжает программу. С
ключом `-semihosting-config enable=on,target=native` эту роль играет сам
QEMU. На плате без подключённого отладчика та же инструкция приводит к
HardFault, а с отладчиком останавливает ядро на каждый вызов. Поэтому
semihosting годится для первых опытов, но не для журнала работающей
прошивки.

</div>

## Один исходник на трёх ядрах

Прошивка `ch00-hello` вызывает функции из раздела [0.4](04-abi.md) и
печатает, на каком ядре работает:

```rust
{{#include ../../firmware/ch00-hello/src/main.rs:cpuid}}
```

```rust
{{#include ../../firmware/ch00-hello/src/main.rs:main}}
```

```console
$ cd firmware
$ cargo run --release -p ch00-hello --target thumbv6m-none-eabi
Cortex-M0 (CPUID = 0x410cc200)
mul_wide = 0xfffffffe00000001
args8    = 0x00000579
keep     = 21
scale    = 4 (биты 0x40800000)

$ cargo run --release -p ch00-hello
Timer with period zero, disabling
Cortex-M3 (CPUID = 0x410fc231)
mul_wide = 0xfffffffe00000001
args8    = 0x00000579
keep     = 21
scale    = 4 (биты 0x40800000)

$ cargo run --release -p ch00-hello --target thumbv7em-none-eabihf
Cortex-M4 (CPUID = 0x410fc240)
mul_wide = 0xfffffffe00000001
args8    = 0x00000579
keep     = 21
scale    = 4 (биты 0x40800000)
```

Строку `Timer with period zero, disabling` печатает сам QEMU: модель
таймера LM3S6965 жалуется, что его не настроили. На прошивку она не
влияет, и `run-demos.sh` её отфильтровывает. В CPUID старший байт `0x41` —
код ARM, биты 15..4 — номер ядра, младшие четыре бита — ревизия.

Результаты на трёх ядрах одинаковые, а код разный. Число `0x579` проверяет
и модульный тест в `code/ch00/src/abi.rs`, так что хост и три ядра
согласны между собой. Как именно получен каждый результат на каждом
ядре, разбирал раздел [0.4](04-abi.md). Посмотрим на один случай вживую.

## gdb

QEMU умеет работать заглушкой gdb: ключ `-gdb tcp::3333` открывает порт
для отладчика, `-S` останавливает ядро до первой инструкции. Отладчик
нужен с поддержкой ARM: `gdb-multiarch` из пакетов Debian и Ubuntu или
`arm-none-eabi-gdb` из тулчейна ARM (приложение D).

```console
$ qemu-system-arm -cpu cortex-m0 -machine microbit -nographic \
>     -semihosting-config enable=on,target=native -S -gdb tcp::3333 \
>     -kernel target/thumbv6m-none-eabi/release/ch00-hello &
$ gdb-multiarch -q target/thumbv6m-none-eabi/release/ch00-hello
Reading symbols from target/thumbv6m-none-eabi/release/ch00-hello...
(gdb) target remote :3333
Remote debugging using :3333
warning: (Internal error: pc 0xc0 in read in CU, but not in symtab.)
...
0x000000c0 in compiler_builtins::float::mul::mul<f16> ()
(gdb) x/2wx 0
0x0:	0x20004000	0x000000c1
(gdb) break ch00_mul_wide
Breakpoint 1 at 0x2e4: file src/abi.rs, line 42.
(gdb) continue
Continuing.

Breakpoint 1, ch00::abi::ch00_mul_wide (a=4294967295, b=4294967295)
    at src/abi.rs:42
42	    a as u64 * b as u64
(gdb) disassemble
Dump of assembler code for function ch00::abi::ch00_mul_wide:
   0x000002dc <+0>:	push	{r7, lr}
   0x000002de <+2>:	add	r7, sp, #0
   0x000002e0 <+4>:	mov	r2, r0
   0x000002e2 <+6>:	movs	r3, #0
=> 0x000002e4 <+8>:	mov	r0, r1
   0x000002e6 <+10>:	mov	r1, r3
   0x000002e8 <+12>:	bl	0x581c <compiler_builtins::int::mul::__aeabi_lmul::__aeabi_lmul>
   0x000002ec <+16>:	pop	{r7, pc}
End of assembler dump.
(gdb) stepi 3
compiler_builtins::int::mul::__aeabi_lmul::__aeabi_lmul ()
    at library/compiler-builtins/compiler-builtins/src/macros.rs:349
warning: 349	library/compiler-builtins/compiler-builtins/src/macros.rs: No such file or directory
(gdb) info registers r0 r1 r2 r3
r0             0xffffffff          -1
r1             0x0                 0
r2             0xffffffff          -1
r3             0x0                 0
(gdb) finish
Run till exit from #0  compiler_builtins::int::mul::__aeabi_lmul::__aeabi_lmul
    () at library/compiler-builtins/compiler-builtins/src/macros.rs:349
ch00::abi::ch00_mul_wide (a=<optimized out>, b=<optimized out>)
    at src/abi.rs:43
43	}
(gdb) info registers r0 r1
r0             0x1                 1
r1             0xfffffffe          -2
```

Исходников `compiler_builtins` на машине нет, поэтому вместо строки
`macros.rs` gdb показывает предупреждение. Ассемблер при этом доступен
всегда: `x/8i $pc` или `layout asm`.

Таблица векторов на месте: начальный SP `0x2000_4000` — конец 16 КиБ ОЗУ
micro:bit, адрес `Reset` `0xc1` с битом Thumb. У ARMv6-M нет инструкции
умножения 32×32→64, поэтому `ch00_mul_wide` расширяет оба аргумента до
`u64` и вызывает `__aeabi_lmul` из `compiler_builtins`. Функция помощи из
EABI принимает два 64-битных числа в парах регистров: первое в `r0:r1`,
второе в `r2:r3`, младшее слово в регистре с меньшим номером. Ровно это и
видно на входе: `0xffffffff` и ноль в каждой паре. Результат
`0xfffffffe_00000001` возвращается в `r0:r1` и совпадает с тем, что
напечатала прошивка.

<div class="box trap">

**Ловушка.** Сразу после подключения gdb уверяет, что ядро стоит в
`compiler_builtins::float::mul::mul<f16>`, и перед этим печатает
полдюжины одинаковых предупреждений:

```text
warning: (Internal error: pc 0xc0 in read in CU, but not in symtab.)
```

Таблица символов говорит другое:

```text
(gdb) info symbol $pc
Reset in section .text
```

Права таблица символов. Функция `mul<f16>` в прошивку не попала:
компоновщик выбросил её вместе с секцией, потому что её никто не
вызывает. Её отладочная информация при этом осталась, а адрес начала
обнулился:

```console
$ llvm-dwarfdump --name='mul<f16>' --show-children=0 target/thumbv6m-none-eabi/release/ch00-hello
0x00009538: DW_TAG_subprogram
              DW_AT_low_pc	(0x00000000)
              DW_AT_high_pc	(0x00000208)
              ...
              DW_AT_name	("mul<f16>")
```

По DWARF функция занимает адреса с 0 по `0x208`, и `Reset` по адресу
`0xc0` попадает внутрь. На `mps2-an386` таблица векторов длиннее, `Reset`
лежит по адресу `0x400`, и путаницы нет. Если gdb показывает
неожиданную функцию рядом с нулевым адресом, проверьте её через
`info symbol` и `x/i $pc`.

</div>

<div class="box c">

**Как в C.** Всё то же самое делают `arm-none-eabi-gcc`, OpenOCD и
`arm-none-eabi-gdb`: Rust не приносит своего отладчика. gdb понимает
отладочную информацию Rust (имена с путями модулей, `enum`, срезы), и
`print` работает с переменными Rust так же, как с переменными C. Для
semihosting в newlib есть `--specs=rdimon.specs`. В Rust его заменяет
крейт `cortex-m-semihosting`.

</div>

## Реальная плата: probe-rs и defmt

Плата в книге не обязательна, но если она есть, понадобятся два
инструмента. Опыты книги на них не опираются, и в CI их нет.

*probe-rs* работает с отладочными адаптерами SWD и JTAG (ST-Link, J-Link,
CMSIS-DAP): прошивает флеш-память, сбрасывает ядро и читает журнал.
Подключается он тем же способом, что QEMU, — строкой `runner` в
`.cargo/config.toml`, так что `cargo run` прошивает плату вместо запуска
эмулятора. Ещё probe-rs умеет работать сервером для gdb и для отладчика
VS Code.

*defmt* (deferred formatting) — библиотека журналирования для
микроконтроллеров. Строки формата не попадают во флеш-память:
макрос кладёт их в отдельную секцию ELF, которая на кристалл не
загружается, а прошивка передаёт только номер строки формата и
аргументы в двоичном виде. Текст собирает хост, читая тот же ELF. Байты
уходят через RTT (Real-Time Transfer): кольцевой буфер в ОЗУ, который
адаптер читает через отладочный порт, не останавливая ядро. Semihosting
останавливает ядро на каждый вызов, RTT — нет. Для RTT нужен отладочный адаптер,
которого у QEMU нет, поэтому книга печатает через semihosting и UART, а
defmt и probe-rs возвращаются в главе 27.

<div class="box check">

**Проверь себя.**

1. Почему регистр CPUID имеет один адрес на Cortex-M0, M3 и M4F, а UART
   у каждой машины свой?
2. Что произойдёт, если прошивку `ch00-hello` записать в плату и
   включить питание без отладчика?
3. У `ch00_mul_wide` два 32-битных аргумента. Почему на Cortex-M0 она
   заполняет перед вызовом четыре регистра?
4. Почему время работы прошивки в QEMU нельзя использовать для оценки
   скорости на плате?

</div>
