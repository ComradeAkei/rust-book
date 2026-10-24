# 29.2. Сборка ядра и первый запуск в QEMU

Чтобы загрузить модуль на Rust, нужно ядро, собранное с `CONFIG_RUST`, и
каталог его сборки: модули компилируются против метаданных крейтов,
которые получились при сборке именно этого ядра, и тем же rustc (раздел
[29.1](01-structure.md)). Глава собирает своё ядро — маленькое, за
несколько минут, — чтобы версия ядра, его конфигурация и компилятор
были под контролем, и запускает его в QEMU: модуль, который роняет ядро,
роняет виртуальную машину, а не вашу.

## Что нужно

| Инструмент | Зачем | Глава |
|---|---|---|
| rustc и `rust-src` | компилятор и исходники `core`, который собирается под цель ядра | 1.97.0 из `rust-toolchain.toml` |
| bindgen | привязки к заголовкам C | 0.72.1: `cargo install --locked bindgen-cli@0.72.1` |
| clang, lld, llvm | `LLVM=1`: ядро тем же LLVM, что у rustc; libclang для bindgen | 18.1.3 (Ubuntu 24.04) |
| make, flex, bison, bc, libelf-dev | обычная сборка ядра | |
| qemu-system-x86, gdb | запуск и отладка | 8.2.2, 15.1 |

Минимальные версии ядро объявляет само, в
`scripts/min-tool-version.sh`: rustc 1.78.0 и bindgen 0.65.1. Проверяет
всё сразу цель `rustavailable`:

```console
$ make -s -C target/ch29-kernel/linux-7.0 LLVM=1 rustavailable
Rust is available!
$ make -s -C target/ch29-kernel/linux-7.0 LLVM=1 rustavailable   # без bindgen
***
*** Rust bindings generator 'bindgen' could not be found.
***
***
*** Please see Documentation/rust/quick-start.rst for details
*** on how to set up the Rust support.
***
```

Если `rustavailable` не довольна, Kconfig не даст включить `CONFIG_RUST`
вовсе: опция зависит от `RUST_IS_AVAILABLE`, и `make olddefconfig` молча
её выключит. Зависимостей у неё и кроме этого немало:

<!-- linux: init/Kconfig -->
```text
config RUST
	bool "Rust support"
	depends on HAVE_RUST
	depends on RUST_IS_AVAILABLE
	select EXTENDED_MODVERSIONS if MODVERSIONS
	depends on !MODVERSIONS || GENDWARFKSYMS
	depends on !GCC_PLUGIN_RANDSTRUCT
	depends on !RANDSTRUCT
	depends on !DEBUG_INFO_BTF || (PAHOLE_HAS_LANG_EXCLUDE && !LTO)
```

`!RANDSTRUCT` читается так: ядро, которое перемешивает поля структур C
при сборке, Rust не поддерживает — привязки описывают поля в порядке
объявления, и раскладка Rust разошлась бы с раскладкой C.

## Конфигурация: tinyconfig и два фрагмента

Ядро дистрибутива собирается час и больше: в нём тысячи драйверов.
Ядру главы нужны процессор, память, последовательный порт, initramfs,
модули и Rust. Скрипт `code/ch29/kernel/build.sh` начинает с
`make tinyconfig` — самой маленькой конфигурации, с которой ядро вообще
собирается, — и накладывает на неё фрагмент `ch29.config`:

```text
{{#include ../../code/ch29/kernel/ch29.config}}
```

Отладочное ядро — то же плюс `ch29-debug.config`: KASAN, kmemleak,
lockdep и проверка сна в атомарном контексте. Оно нужно разделам
[29.4](04-memory.md) и [29.5](05-pin-init.md) и задаче
[29.A](10-tasks.md).

```text
{{#include ../../code/ch29/kernel/ch29-debug.config}}
```

После `merge_config.sh` и `make olddefconfig` скрипт проверяет, что
каждая заказанная опция включена, а выключенная — выключена. Это не
перестраховка: Kconfig выкидывает опцию с неудовлетворёнными
зависимостями без единого слова. Так было с `CONFIG_KASAN` без
`CONFIG_KASAN_VMALLOC` — пропал `CONFIG_VMAP_STACK`, у которого в
зависимостях `!KASAN || KASAN_HW_TAGS || KASAN_VMALLOC`, — и с ядром без
`CONFIG_ACPI`, которое видело в QEMU один процессор из двух: о втором
ядро узнаёт из таблиц ACPI, а разбора старых таблиц MP
(`CONFIG_X86_MPPARSE`) в tinyconfig нет.

```console
$ bash code/ch29/kernel/build.sh
загрузка https://archive.ubuntu.com/ubuntu/pool/main/l/linux/linux_7.0.0.orig.tar.gz
Rust is available!
сборка: plain → /…/target/ch29-kernel/build
plain: 217 с, 778M, bzImage 2.3M
сборка: debug → /…/target/ch29-kernel/build-debug
debug: 250 с, 985M, bzImage 4.1M
```

Время — в четыре потока; в конфигурации 674 включённые опции у
обычного ядра и 699 у отладочного. Исходники — архив ядра 7.0 без
патчей дистрибутива, контрольную сумму скрипт сверяет. Ядро собирается
с `LLVM=1`: clang вместо gcc, lld вместо ld, — тогда C и Rust
компилирует один и тот же LLVM, а bindgen разбирает заголовки тем же
clang, которым они компилируются. Сборка gcc, по словам
`Documentation/rust/quick-start.rst`, «работает для некоторых
конфигураций, но пока очень экспериментальна».

<div class="box hood">

**Под капотом: что собирается из Rust.** Строки журнала сборки,
которые относятся к Rust:

```text
  BINDGEN rust/bindings/bindings_generated.rs
  BINDGEN rust/bindings/bindings_helpers_generated.rs
  CC      rust/helpers/helpers.o
  BINDGEN rust/uapi/uapi_generated.rs
  EXPORTS rust/exports_helpers_generated.h
  RUSTC L rust/uapi.o
  RUSTC L rust/bindings.o
  EXPORTS rust/exports_bindings_generated.h
  RUSTC L rust/kernel.o
```

Порядок — порядок зависимостей из раздела [29.1](01-structure.md):
привязки, помощники на C, крейты Rust (`L` — библиотека), списки
экспорта. Журнал — от повторной сборки: `core`, `compiler_builtins`,
`pin_init` и процедурные макросы собраны раньше и не пересобирались.

</div>

## Внешний модуль

Модули главы собираются вне дерева ядра — так же, как собирают свой
драйвер на C. Каталог модуля — исходники и файл `Kbuild` со списком
объектов:

```make
{{#include ../../code/ch29/modules/Kbuild}}
```

По расширению исходника Kbuild сам решает, чем компилировать объект:
`ch29_hello.rs` — rustc, `ch29_errors_c.c` — clang. Сборку запускают из
каталога сборки ядра: `M=` — каталог модуля, `MO=` — куда класть
объекты, чтобы не мусорить в исходниках. Обвязка главы делает то же из
теста:

```rust,ignore
{{#include ../../code/ch29/src/host.rs:make}}
```

```console
$ make -C target/ch29-kernel/build M=$PWD MO=out LLVM=1
  GEN     Makefile
  RUSTC [M] ch29_hello.o
  MODPOST Module.symvers
  CC [M]  ch29_hello.mod.o
  CC [M]  .module-common.o
  LD [M]  ch29_hello.ko
```

`RUSTC [M]` — модуль на Rust компилируется в один объектный файл, как
файл C. Дальше всё как у C-модуля: `modpost` сверяет неразрешённые
символы с экспортом ядра, `ch29_hello.mod.c` — сгенерированный на C
список зависимостей и `vermagic`, а `ld` собирает `.ko`.

`CLIPPY=1` в той же команде заменяет rustc на `clippy-driver` с линтами,
которые ядро считает обязательными: `// SAFETY:` у каждого блока
`unsafe`, раздел `# Safety` у каждой `unsafe fn` и запрет на API новее
минимальной версии rustc ядра (1.78) — даже если сам компилятор новее.
Все модули главы и решения задач собираются с `CLIPPY=1`, и тесты
требуют, чтобы предупреждений не было.

## Машина: initramfs и init на Rust

Внутри QEMU нет ни диска, ни оболочки. Ядро получает образ initramfs —
архив cpio, который оно распаковывает в память как корневую файловую
систему, — и запускает из него `/init` процессом 1. Образ собирает
утилита ядра `usr/gen_init_cpio` по списку файлов:

```rust,ignore
{{#include ../../code/ch29/src/host.rs:initramfs}}
```

`/init` — программа на Rust из крейта главы, собранная статически
(`-C target-feature=+crt-static`): в образе нет динамического
загрузчика и libc. Ни `insmod`, ни `mount` в образе тоже нет — init
делает всё системными вызовами сам, через nix из главы
[28](../ch28/index.md). Сначала файловые системы:

```rust,ignore
{{#include ../../code/ch29/src/guest.rs:setup}}
```

Потом загрузка модулей — тот же вызов, что у `insmod`:

```rust,ignore
{{#include ../../code/ch29/src/guest.rs:insmod}}
```

Журнал ядра init читает из `/dev/kmsg`: каждое чтение — одна запись с
уровнем, номером и временем, а байты вне ASCII ядро записывает как
`\xNN` — и кириллицу из `pr_info!` тоже. Разбор:

```rust,ignore
{{#include ../../code/ch29/src/kmsg.rs:parse}}
```

Сеанс печатает записи журнала после каждой команды, как `dmesg -x`:
уровень, двоеточие, текст. Так выглядит весь вывод в этой главе:

```text
# insmod /lib/ch29_hello.ko
warn:   ch29_hello: loading out-of-tree module taints kernel.
info:   ch29_hello: привет #1
```

<div class="box trap">

**Ловушка: процесс 1 не должен завершаться.** Если init выходит, ядро
отвечает паникой `Attempted to kill init!`. Поэтому сценарий кончается
не выходом, а `reboot(RB_AUTOBOOT)` — и QEMU, запущенный с `-no-reboot`,
на перезагрузку завершается. Перед `reboot`, а также перед `insmod`,
`rmmod` и опытом в дочернем процессе init вызывает `tcdrain` и ждёт,
пока последовательный порт допечатает вывод. `write` в консоль
возвращается, как только байты легли в буфер tty, а сообщения ядра
идут на порт мимо этого буфера. Без ожидания они рвут строку сеанса
посередине. И поэтому опыты, которые убивают процесс (oops в модуле,
раздел [29.9](09-oops.md)), init выполняет в дочернем процессе.

</div>

## QEMU

```rust,ignore
{{#include ../../code/ch29/src/host.rs:qemu}}
```

Командная строка ядра — `console=ttyS0 loglevel=1 panic=-1 -- сценарий`:
консоль на последовательный порт, который у QEMU с `-nographic` —
stdout; на консоль — только сообщения уровня `KERN_EMERG` (остальное
init читает из журнала сам, и строки не перемешиваются); при панике —
сразу перезагрузка, то есть выход QEMU. Всё после `--` ядро не
разбирает, а передаёт init в `argv`: так тест выбирает сценарий.

Под TCG загрузка с одним опытом занимает около четырёх секунд, под KVM
— меньше. Каждый тест грузит ядро заново, и опыты не влияют друг на
друга: oops или утечка в одном не достаются следующему.

<div class="box check">

**Проверь себя.**

1. Что нужно, кроме исходников модуля, чтобы собрать модуль на Rust для
   уже работающего ядра, и почему одних заголовков ядра мало?
2. Что произойдёт, если убрать из `ch29.config` строку `CONFIG_ACPI=y`?
   А если убрать проверку из `build.sh`?
3. Зачем init монтирует devtmpfs, а не создаёт узлы устройств в
   initramfs, как `/dev/console`?

</div>
