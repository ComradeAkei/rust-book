# 25.6. Во flash или в ОЗУ

## Что видит компоновщик

Раздел [8.8](../ch08/08-sections.md) разобрал, куда попадают `static` и
`const`: неизменяемый `static` без внутренней изменяемости — в
`.rodata`, во flash; изменяемый с ненулевым начальным значением — в
`.data`, то есть в ОЗУ, а его начальное значение — ещё и во flash;
изменяемый, равный нулю, — в `.bss`, только в ОЗУ. Здесь — сколько это
стоит на прошивке, где таблицы посчитаны `const fn`, и куда они
переезжают от одного слова `mut`.

Код запуска cortex-m-rt до `main` копирует `.data` из flash в ОЗУ и
обнуляет `.bss`. Границы этих областей — символы скрипта компоновки
`link.x`, и прошивка может их прочитать:

```rust
{{#include ../../firmware/ch25-flash/src/main.rs:linker}}
```

Символ компоновщика — адрес без объекта: `__sdata` не переменная типа
`u8`, а метка начала `.data`. Читать из неё нельзя, брать адрес можно, и
`&raw const __sdata` даёт его без промежуточной ссылки.

```rust
{{#include ../../firmware/ch25-flash/src/main.rs:startup}}
```

## Прошивка ch25-flash

Кроме таблиц из `code/ch25` — классов байтов `CLASSES` (раздел
[25.2](02-const-fn.md)) и обратных `RECIP` (раздел [25.5](05-tables.md))
— у прошивки два своих объекта в ОЗУ:

```rust
{{#include ../../firmware/ch25-flash/src/main.rs:ram}}
```

Делитель UART0 из раздела [25.2](02-const-fn.md) прошивка записывает в
регистры PL011 и читает обратно:

```rust
{{#include ../../firmware/ch25-flash/src/main.rs:uart}}
```

```console
$ cd firmware && cargo run --release -p ch25-flash
.data: 4 байт в ОЗУ с 0x20000000, образ во flash с 0x00001fbc
.bss:  72 байт в ОЗУ с 0x20000004, обнулены до main
static CLASSES         0x000019c6  flash   256 байт
static RECIP           0x00001ac8  flash  1024 байт
static BOOTS (.data)   0x20000000  ОЗУ       4 байт
static mut LOG (.bss)  0x2000000c  ОЗУ      64 байт
UART0 при 16 МГц, 115200 бод: IBRD = 8, FBRD = 44, ошибка -799 ppm
отсчёт 3000 / 7: таблица 428, деление 428
таблица и деление совпали: 4 из 4
BOOTS = 2
```

Разберём построчно.

- `.data` — 4 байта: это `BOOTS`, атомик с начальным значением 1. Код
  запуска скопировал эти 4 байта с адреса 0x1fbc во flash в 0x2000_0000.
  `main` начинает с `BOOTS` = 1, а `BOOTS = 2` в конце — результат
  `store`.
- `.bss` — 72 байта: 64 байта `LOG` и 8 байт `HSTDOUT` из
  `cortex-m-semihosting` (раздел [8.8](../ch08/08-sections.md)), который
  `llvm-nm` показывает по адресу 0x2000_0004, перед `LOG`. Всё это код
  запуска обнулил, во flash для них ничего нет.
- `CLASSES` и `RECIP` — 1280 байт во flash и ни байта в ОЗУ. Кода,
  который их строит, в прошивке нет: `byte_classes` и `reciprocals`
  выполнил компилятор.
- UART0 получил 8 и 44 — числа из константы `UART0`. Деление на частоту
  тоже выполнил компилятор.

На Cortex-M0 (`--target thumbv6m-none-eabi`, micro:bit) вывод тот же,
кроме адресов и строки UART: у nRF51822 по адресу UART0 lm3s6965 другая
периферия, поэтому прошивка проверяет номер ядра в регистре CPUID и
пишет в UART0 только на Cortex-M3. Деление на M0 — вызов
`__aeabi_uidiv`, таблица — умножение, и результаты совпали так же.

## Одно слово mut

Возможность `ram-table` добавляет ту же таблицу обратных в `static mut`:

```console
$ cargo run --release -p ch25-flash --features ram-table
.data: 1028 байт в ОЗУ с 0x20000000, образ во flash с 0x00002080
.bss:  72 байт в ОЗУ с 0x20000404, обнулены до main
…
static mut RECIP_RAM   0x20000004  ОЗУ    1024 байт
```

Таблица посчитана той же `const fn` и так же при компиляции. Но
`static mut` можно менять, и flash для него не годится: таблица
переехала в `.data`. Теперь она занимает 1024 байта ОЗУ, её образ — ещё
1024 байта во flash, а код запуска копирует её при каждом сбросе:

```console
$ cargo build --release -p ch25-flash
$ llvm-size target/thumbv7m-none-eabi/release/ch25-flash
   text	   data	    bss	    dec	    hex	filename
   8124	      4	     72	   8200	   2008	target/thumbv7m-none-eabi/release/ch25-flash
$ cargo build --release -p ch25-flash --features ram-table
$ llvm-size target/thumbv7m-none-eabi/release/ch25-flash
   text	   data	    bss	    dec	    hex	filename
   8320	   1028	     72	   9420	   24cc	target/thumbv7m-none-eabi/release/ch25-flash
```

ОЗУ под данные выросло с 76 байт до 1100. На micro:bit с его 16 КиБ ОЗУ
это 6 % памяти за таблицу, которую никто не меняет. Колонка `text`
выросла на 196 байт: печать адреса новой таблицы в `main`. А образ
`.data` во flash (колонка `data` — его размер) — ещё 1024 байта сверх
этого.

<div class="box c">

**Как в C.** Ровно та же ошибка в C — забытый `const`:
`uint32_t recip[256] = {…};` — 1 КиБ ОЗУ и копирование при каждой
загрузке. Разница в умолчании: в C неизменяемость нужно попросить, в
Rust изменяемость. `static` без `mut` и без `Cell`, атомиков и
мьютексов внутри всегда во flash, а `static mut` бросается в глаза
каждым `unsafe` при обращении.

</div>

<div class="box hood">

**Под капотом.** Код запуска cortex-m-rt 0.7.7 — обработчик `Reset`
в `global_asm!`: цикл обнуления слов от `__sbss` до `__ebss` и цикл
копирования от `__sidata` в `__sdata`…`__edata`. Он на ассемблере, а не
на Rust, потому что до конца этих циклов код на Rust выполнять нельзя:
компилятор вправе считать, что каждый `static` уже имеет своё начальное
значение. Свой код запуска без cortex-m-rt — глава 26.

</div>

<div class="box check">

**Проверь себя.** Что будет с размером `.data` и `.bss`, если `BOOTS`
начать с нуля? Если `LOG` объявить `static LOG: [u8; 64] = [0; 64];` без
`mut`? Проверьте на прошивке.

</div>
