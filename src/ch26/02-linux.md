# 26.2. Программа для Linux без libc

## Что происходит до main

Программа на C начинается не с `main`. Компоновщик ставит точкой входа
`_start` из `crt1.o` — файла запуска libc. `_start` достаёт со стека argc,
argv и окружение и вызывает `__libc_start_main`: та настраивает локальную
память потока, регистрирует обработчики `atexit`, вызывает конструкторы из
`.init_array`, затем `main`, и с его результатом — `exit`. Программа на
Rust со `std` идёт тем же путём и перед `main` делает своё: `std` ставит
обработчик переполнения стека и игнорирование `SIGPIPE`.

Без libc всего этого нет. Ядро загружает ELF, кладёт на стек аргументы
и передаёт управление по адресу точки входа — и с этого момента
программа одна. Пишем её: `code/ch26/start`, отдельный крейт со своим
workspace, чтобы его профили не трогали остальные крейты книги.

```rust,ignore
{{#include ../../code/ch26/start/build.rs:build}}
```

`-nostartfiles` убирает `crt1.o`, `crti.o` и `crtn.o`: точку входа мы
дадим сами. `-static` — без динамического загрузчика. Саму libc
подключать не нужно: rustc передаёт компоновщику `-nodefaultlibs`
всегда, а `libc` в программу приводит только `std`.

## Точка входа

```rust,ignore
{{#include ../../code/ch26/start/src/main.rs:start}}
```

`_start` — [naked-функция][g-naked-function] (раздел [19.2](../ch19/02-naked.md)): у неё нет
[пролога][g-prologue], и компилятор не трогает стек до первой инструкции, которую
написали мы. Обычная функция начала бы с `push`, и `rsp` перестал бы
указывать на argc. `rbp = 0` отмечает конец цепочки кадров для
отладчика. `call` вместо `jmp` — ради выравнивания: [ABI][g-abi] x86-64 требует,
чтобы в точке вызова `rsp` был кратен 16, то есть при входе в функцию,
после адреса возврата, `rsp + 8` кратно 16. Ядро даёт стек, кратный 16,
и `call` кладёт ровно восемь байт. `ud2` — ловушка на случай, если
функция `-> !` всё-таки вернётся.

<div class="box ref">

**Справка: стек нового процесса.** Ядро (`create_elf_tables` в
`fs/binfmt_elf.c`) кладёт на стек снизу вверх: argc; argv[0..argc] и
NULL; указатели окружения и NULL; *вспомогательный вектор* (auxiliary
vector, auxv) — пары «тип, значение» до `AT_NULL` = 0; выше — сами строки
и 16 случайных байт. Формат закреплён в System V ABI для x86-64, раздел
3.4.1. Через auxv ядро сообщает размер страницы (`AT_PAGESZ`), точку входа
(`AT_ENTRY`), адрес заголовков программы (`AT_PHDR`), адрес [vDSO][g-vdso]
(`AT_SYSINFO_EHDR`) и адрес 16 случайных байт (`AT_RANDOM`): из них
glibc берёт канарейку стека. `LD_SHOW_AUXV=1 /bin/true` печатает auxv
любой динамической программы.

</div>

```rust,ignore
{{#include ../../code/ch26/start/src/main.rs:main}}
```

```console
$ cd code/ch26/start && cargo build --release
$ env -i A=1 B=2 target/release/ch26-start один два
argc = 3
argv[0] = target/release/ch26-start
argv[1] = один
argv[2] = два
переменных окружения: 2
AT_PAGESZ = 4096
AT_ENTRY = 0x0000000000201410, _start = 0x0000000000201410
AT_RANDOM: 16 случайных байт по адресу 0x00007ffff1b173c9
rsp при входе в _start: 0x00007ffff1b17210, кратно 16
```

`AT_ENTRY` совпал с адресом `_start` — ядро сообщает программе её же
точку входа. Адреса стека от запуска к запуску другие: ASLR. Тест
`rustc_start_reads_the_initial_stack` запускает программу с двумя
аргументами и окружением из двух переменных и сверяет первые строки и
кратность.

## Системные вызовы

Вывод — без `write` из libc. Системный вызов на x86-64 — инструкция
`syscall`: номер в `rax`, аргументы в `rdi`, `rsi`, `rdx`, `r10`, `r8`,
`r9`, результат — в `rax`.

```rust,ignore
{{#include ../../code/ch26/src/sys.rs:syscall}}
```

`syscall` сохраняет адрес возврата в `rcx`, а `rflags` — в `r11`, и оба
регистра после вызова испорчены: `lateout(…) _` говорит об этом
компилятору. `nostack` — инструкция не пишет в стек. `inlateout("rax")`
— номер входит в `rax`, а результат выходит оттуда же. Это не
`options(nomem)`: ядро читает и пишет память по нашим указателям, и
компилятор должен считать, что `asm!` с ней работает.

```rust,ignore
{{#include ../../code/ch26/src/sys.rs:wrappers}}
```

Ошибку ядро возвращает отрицательным числом от −4095 до −1 — это
`-errno`. Переменной `errno` у ядра нет: её заводит libc, перекладывая
туда этот код и возвращая −1. Без libc обёртка вольна вернуть то, что
удобнее, — здесь `Result<usize, Errno>`. Тесты `sys` пишут во временный
файл через свой `write`, получают `EBADF` на дескрипторе −1 и `EFAULT`
(−14) на адресе 8, который не отображён: ядро проверяет указатели само
и не падает на чужой ошибке. `exit` — это `exit_group` (231), а не
`exit` (60): тот завершает только вызвавший поток.

## Почему без LTO не компонуется

Профили программы:

```toml
{{#include ../../code/ch26/start/Cargo.toml:profile}}
```

`panic = "abort"` обязателен: раскрутка стека — часть `std`, без неё
паника может только остановить программу. Обработчик паники завершает
процесс с тем же кодом 101, что и необработанная паника в программе со
`std`:

```rust,ignore
{{#include ../../code/ch26/start/src/main.rs:panic}}
```

А `lto = true` — уже не выбор. Готовая `core` для
x86_64-unknown-linux-gnu собрана с раскруткой, и без LTO:

```text
$ CARGO_PROFILE_RELEASE_LTO=false cargo build --release
error: linking with `cc` failed: exit status: 1
  = note: rust-lld: error: undefined symbol: rust_eh_personality
          >>> referenced by core.940e698046960fd5-cgu.0
```

[`rust_eh_personality`][g-personality] — функция, которую раскрутчик зовёт на каждом
кадре, чтобы узнать, есть ли в нём что убирать. Её даёт `std`. Код
`core` на неё ссылается, хотя при `panic = "abort"` раскрутки не будет
никогда, а с LTO ссылка исчезает: тест
`rustc_start_without_lto_needs_personality` проверяет оба исхода. В
отладочном профиле без оптимизаций компоновщик требует ещё и `memset`:
обнуление массива при `opt-level = 0` — вызов (раздел
[26.5](05-memcpy.md)), поэтому и там `opt-level = "s"`.

<div class="box hood">

**Под капотом.** Для Cortex-M этой проблемы нет: в описании цели
thumbv7m-none-eabi стоит `"panic-strategy": "abort"`
(`rustc +nightly -Z unstable-options --print target-spec-json`), и
прошивки компонуются без LTO и в отладочном профиле. У
x86_64-unknown-linux-gnu стратегия по умолчанию — раскрутка. Вторая дорога
для Linux — пересобрать `core` под свой профиль ключом `-Z build-std=core`
на nightly; эта книга держится стабильного компилятора.

</div>

## Статический и динамический

```console
$ file target/release/ch26-start
ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked, …
$ CH26_DYNAMIC=1 cargo build --release && file target/release/ch26-start
ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked,
interpreter /lib64/ld-linux-x86-64.so.2, …
```

С `-static` — `ET_EXEC` по фиксированному адресу, без `PT_INTERP` и
`PT_DYNAMIC`. Без него rustc собирает позиционно-независимую программу
(`-pie`), и компоновщик вписывает интерпретатор: ядро отобразит программу
по случайному адресу, рядом — `ld-linux-x86-64.so.2`, и передаст
управление ему, а тот применит перемещения и прыгнет на наш `_start`.
Библиотек загружать нечего, но программа работает и так — тесты
`rustc_start_is_static_exec` и
`rustc_start_without_static_gets_interpreter` проверяют оба варианта по
заголовкам ELF.

## Своя strlen

Есть ещё одна функция, которую пришлось написать:

```rust,ignore
{{#include ../../code/ch26/start/src/main.rs:strlen}}
```

Длину строки C для `argv[i]` программа считала циклом `while`, и
компоновщик потребовал `strlen`: LLVM узнал в цикле идиому и заменил
его вызовом. На x86-64 Linux такие функции даёт libc, и
`compiler_builtins` для этой цели их не определяет. Как и откуда
компилятор берёт эти вызовы — раздел [26.5](05-memcpy.md).

<div class="box trap">

**Ловушка.** Выравнивание стека — часть ABI, и нарушает его не
компилятор, а `_start`. Возможность `misalign` добавляет в `_start`
одну инструкцию `sub rsp, 8` перед `call`:

```console
$ cargo build --release --features misalign && target/release/ch26-start
argc = Segmentation fault
$ dmesg | tail -1
traps: ch26-start[22415] general protection fault ip:202037 sp:7ffc28465e88 error:0
```

`argc = ` напечаталось, а на числе — авария. По адресу 0x202037 — функция
`num`:

```text
num:
	sub	rsp, 0x18
	xorps	xmm0, xmm0
	movaps	xmmword ptr [rsp], xmm0
```

`movaps` пишет 16 байт и требует адреса, кратного 16. Компилятор
обнулил буфер на стеке одной векторной записью, рассчитывая на
выравнивание, которое обещает ABI. Тест
`rustc_observed_misaligned_stack_faults_on_movaps` фиксирует сигнал 11.
Наблюдение, а не гарантия: другой код мог бы и не использовать `movaps`
и проработать со сдвинутым стеком до первого `printf` с `double`.

</div>

<div class="box check">

**Проверь себя.** Программа печатает и завершается, а `std` в ней нет.
Что сломается, если в ней создать поток через системный вызов `clone`
и вызвать в нём `panic!`? А `exit` (60) вместо `exit_group` в главном
потоке без других потоков — отличается ли результат?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-abi]: ../appendix-f.md#abi "ABI (application binary interface): Двоичный интерфейс платформы: соглашение о вызовах, раскладка типов в памяти, имена символов."
[g-naked-function]: ../appendix-f.md#naked-function "Naked-функция (naked function): Функция без пролога и эпилога, сгенерированных компилятором: её тело — ровно инструкции naked_asm!, и кадр стека, сохранение регистров и возврат пишет программист."
[g-personality]: ../appendix-f.md#personality "Функция личности (personality routine): Функция, которую раскрутчик вызывает для каждого кадра: по таблице LSDA в секции .gcc_except_table она решает, есть ли в кадре посадочная площадка."
[g-prologue]: ../appendix-f.md#prologue "Пролог и эпилог (prologue, epilogue): Начальные и конечные команды функции: пролог сохраняет callee-saved регистры и адрес возврата и выделяет кадр стека, эпилог восстанавливает их и возвращается."
[g-vdso]: ../appendix-f.md#vdso "vDSO (virtual dynamic shared object): Маленькая разделяемая библиотека, которую ядро Linux отображает в каждый процесс вместе со страницей данных [vvar]."
