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

## Debian и Ubuntu

```console
$ sudo apt install qemu-system-arm gdb-multiarch llvm
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ cargo install mdbook
```

## Общая часть

```console
$ cd rust-book
$ rustc --version                 # rustup поставит 1.97.0 из rust-toolchain.toml
$ rustup toolchain install nightly --component miri
$ cargo +nightly miri setup
$ cargo install cargo-show-asm
```

Проверка, что всё на месте:

```console
$ cargo test                      # примеры глав, решения, проект
$ cd firmware && bash run-demos.sh
ok    UART PL011, таблица размеров на M3
ok    M3: LDR по нечётному адресу
...
$ cd .. && mdbook build            # книга в каталоге book/
```

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

