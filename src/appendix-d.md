# Приложение D. Установка окружения

Arch Linux и Debian/Ubuntu: rustup, целевые платформы, qemu-system-arm,
gdb-multiarch, probe-rs.

Всё, что нужно для книги: Rust с тремя целями Cortex-M, nightly с Miri,
QEMU, отладчик, `cargo-show-asm` и mdBook. Версия stable закреплена файлом
`rust-toolchain.toml`, поэтому rustup сам поставит её при первой сборке.

## Arch Linux

```console
$ sudo pacman -S --needed rustup qemu-system-arm arm-none-eabi-gdb llvm mdbook
$ rustup default stable
```

`llvm` даёт `llvm-objdump` и `llvm-nm` с поддержкой ARM. Вместо
`arm-none-eabi-gdb` подойдёт любой gdb, собранный со всеми архитектурами.

Если раньше стоял пакет `rust`, pacman предложит его удалить: `rustup` с
ним конфликтует и сам предоставляет `rust` и `cargo` для зависимостей
других пакетов. Компилятор C для примеров [приложения A](appendix-a.md)
входит в `base-devel`, а для сверки раскладки с компилятором C для
Cortex-M пригодится пакет `arm-none-eabi-gcc`.

## Debian и Ubuntu

```console
$ sudo apt install qemu-system-arm gdb-multiarch llvm
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ cargo install mdbook
```

Компилятор C для примеров [приложения A](appendix-a.md) — пакет `gcc`,
UBSan приходит вместе с ним как зависимость `libubsan1`. Компилятор C для
Cortex-M — пакет `gcc-arm-none-eabi`.

## Общая часть

```console
$ cd rust-book
$ rustc --version                 # rustup поставит 1.97.0 из rust-toolchain.toml
$ rustup toolchain install nightly --component miri
$ cargo +nightly miri setup
$ cargo install cargo-show-asm
```

`cargo miri setup` сам доставит компонент `rust-src`, если его нет. Тот же
компонент нужен для `-Zbuild-std`, с которым в [приложении C](appendix-c.md)
запускается ThreadSanitizer.

Проверка, что всё на месте:

```console
$ cargo test                      # примеры глав, решения, проект
$ cd firmware && bash run-demos.sh
ok    UART PL011, таблица размеров на M3
ok    M3: LDR по нечётному адресу
...
$ cd .. && mdbook build            # книга в каталоге book/
```

Примеры на C из [приложения A](appendix-a.md) проверяются отдельно:
`bash code/appendix-a/c/run.sh` печатает `ok` для каждой проверки.

<div class="box trap">

**Ловушка.** Не задавайте переменную `RUSTFLAGS` при сборке прошивок. Она
не дополняет, а заменяет `rustflags` из `firmware/.cargo/config.toml`, и
вместе с ними пропадает `-Tlink.x`, скрипт компоновки `cortex-m-rt`.
Прошивка соберётся с единственным предупреждением:

```text
warning: linker stderr: rust-lld: cannot find entry symbol _start; not setting start address
```

В ELF-файле не окажется ни таблицы векторов, ни кода: без точки входа
компоновщик выбросил все секции как недостижимые, и `llvm-objdump -h`
покажет одну отладочную информацию. QEMU сразу остановится:

```text
qemu: fatal: Lockup: can't escalate 3 to HardFault (current priority -1)
```

Нужен дополнительный флаг — допишите его в `rustflags` в `config.toml`.

</div>

Для реальных плат понадобится ещё `probe-rs` (`cargo install probe-rs-tools`),
но все задачи книги решаются в QEMU.

## Отладка в QEMU

QEMU сам служит gdb-сервером. Аргументы после `--` cargo передаёт раннеру
из `firmware/.cargo/config.toml`, поэтому набирать команду QEMU целиком не
нужно:

```console
$ cd firmware
$ cargo run -p ch06-uart -- -S -gdb tcp::3333
```

`-gdb tcp::3333` открывает порт для отладчика, а `-S` держит процессор
остановленным, пока отладчик не скажет `continue`. Во втором терминале:

```console
$ gdb-multiarch -q target/thumbv7m-none-eabi/debug/ch06-uart    # на Arch: arm-none-eabi-gdb
(gdb) target remote :3333
(gdb) info registers pc
pc             0x400               0x400 <Reset>
(gdb) break main
Breakpoint 1 at 0x802: file ch06-uart/src/main.rs, line 40.
(gdb) continue
Breakpoint 1, ch06_uart::__cortex_m_rt_main_trampoline () at ch06-uart/src/main.rs:40
40	#[entry]
(gdb) step
ch06_uart::__cortex_m_rt_main () at ch06-uart/src/main.rs:43
43	    let mut out = Uart(unsafe { Pl011::at(Pl011::UART0_BASE) });
(gdb) kill
```

Процессор стоит на `Reset`, обработчике сброса из `cortex-m-rt`. Что
происходит между `Reset` и `main`, разбирается в [главе 0](ch00/06-qemu.md).
Символ `main` — переходник, который генерирует атрибут `#[entry]`,
поэтому точка останова встаёт на строку с атрибутом, а `step` приводит в
тело вашей функции. Отладочный профиль прошивок собран с
`opt-level = "s"`, и `next` иногда возвращается на уже пройденную строку:
код переставлен оптимизатором. После `kill` QEMU завершается с сообщением
«QEMU: Terminated via GDBstub».

<div class="box trap">

**Ловушка: «Internal error» в gdb.** gdb 15.1 при остановке в начале флеша
печатает

```text
warning: (Internal error: pc 0x400 in read in CU, but not in symtab.)
0x00000400 in core::num::imp::flt2dec::strategy::grisu::format_exact_opt ()
```

и называет адрес `0x400` именем функции из `core`, хотя там лежит `Reset`.
Таблица строк отладочной информации `core` ссылается на этот адрес, что
видно по `llvm-dwarfdump --lookup=0x400` для того же ELF-файла. Верное имя
даёт таблица символов: `info registers pc` печатает `<Reset>`. Точки
останова и пошаговое выполнение в вашем коде работают как обычно.

</div>

## Частые ошибки

**`can't find crate for core`.** Цели Cortex-M ставятся для версии из
`rust-toolchain.toml`, но не для nightly. Сборка вида
`cargo +nightly build --target thumbv7m-none-eabi` падает:

```text
error[E0463]: can't find crate for `core`
  |
  = note: the `thumbv7m-none-eabi` target may not be installed
  = help: consider downloading the target with `rustup target add thumbv7m-none-eabi`
```

Совет из сообщения в каталоге книги не поможет: `rustup target add`
добавит цель к 1.97.0 из `rust-toolchain.toml` и ответит, что она уже
стоит. Тулчейн нужно назвать явно:
`rustup target add --toolchain nightly thumbv7m-none-eabi`.

**`the 'miri' component ... is not available`.** Команда `cargo miri test`
без `+nightly` в каталоге книги обращается к 1.97.0, где Miri нет:

```text
error: the 'miri' component which provides the command 'cargo-miri' is not available for the '1.97.0-x86_64-unknown-linux-gnu' toolchain
```

Miri всегда запускается как `cargo +nightly miri ...`.

**`mixing -Zsanitizer will cause an ABI mismatch`.** ThreadSanitizer и
MemorySanitizer требуют стандартную библиотеку, собранную с той же
инструментацией. Добавьте `-Zbuild-std` и явную цель
`--target x86_64-unknown-linux-gnu`, как в [приложении C](appendix-c.md).
AddressSanitizer собирается и без этого.

## Версии, на которых проверена книга

| Инструмент | Версия | Зачем нужен |
|---|---|---|
| rustc, cargo, clippy, rustfmt | 1.97.0 | вся книга, версия закреплена в `rust-toolchain.toml` |
| nightly | 1.101.0-nightly от 2026-10-20 | Miri, `-Zprint-type-sizes`, санитайзеры, коды ошибок в doc-тестах |
| QEMU | 8.2.2 | прошивки |
| gdb-multiarch | 15.1 | отладка прошивок |
| gcc | 13.3 | C-примеры приложения A под UBSan |
| arm-none-eabi-gcc | 13.2.1 | сверка раскладки с C для Cortex-M |
| cargo-show-asm | 0.2.63 | ассемблер в главах |
| mdBook | 0.5.4 | сборка книги |

Про doc-тесты: код ошибки в `compile_fail,E0277` rustdoc сверяет только
на nightly, а stable проверяет лишь то, что пример не компилируется.
Поэтому CI прогоняет doc-тесты ещё раз командой `cargo +nightly test --doc`.
Сообщения компилятора, адреса в выводе QEMU и номера строк в выводе Miri
зависят от версий, и на более новых могут немного отличаться.
