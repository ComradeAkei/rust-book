# 8.5. Сценарий сборки build.rs

## Программа, которая работает до компиляции

Если в корне пакета лежит `build.rs`, Cargo сначала компилирует его как
обычную программу, запускает её и только потом компилирует сам крейт.
Сценарий может читать файлы, генерировать исходники, собирать C-код и
сообщать Cargo, что нужно компилятору и компоновщику. Общается он через
строки в стандартном выводе:

| Строка | Что делает |
|---|---|
| `cargo::rerun-if-changed=путь` | перезапускать сценарий, только если изменился этот файл |
| `cargo::rustc-cfg=имя` | включить условие `#[cfg(имя)]` для крейта |
| `cargo::rustc-check-cfg=cfg(имя)` | объявить это условие допустимым (раздел [8.3](03-cfg.md)) |
| `cargo::rustc-link-search=каталог` | где компоновщику искать библиотеки и скрипты |
| `cargo::rustc-link-lib=имя` | подключить библиотеку, как `-lимя` |
| `cargo::warning=текст` | показать предупреждение при сборке |

А Cargo передаёт сценарию сведения через переменные окружения: каталог
для результатов `OUT_DIR`, цель (`TARGET`, `CARGO_CFG_TARGET_ARCH` и
остальные `CARGO_CFG_*`), включённые возможности (`CARGO_FEATURE_STD`),
профиль.

## Таблица из CSV

Описание датчиков станции хранится в файле, который удобно править и
проверять отдельно от кода:

```text
{{#include ../../code/ch08/sensors.csv}}
```

Сценарий `code/ch08/build.rs` превращает его в статическую таблицу и
функцию поиска:

```rust
{{#include ../../code/ch08/build.rs:sensors}}
```

```rust
{{#include ../../code/ch08/build.rs:main}}
```

Результат лежит в `target/debug/build/ch08-<хеш>/out/sensors.rs`:

```rust,ignore
// Сгенерировано build.rs из sensors.csv. Не редактировать.

pub static SENSORS: [SensorInfo; 4] = [
    SensorInfo { id: 1, name: "board_temp", unit: "mC", scale: 10 },
    SensorInfo { id: 2, name: "ext_temp", unit: "mC", scale: 10 },
    SensorInfo { id: 7, name: "pressure", unit: "Pa", scale: 25 },
    SensorInfo { id: 12, name: "battery", unit: "mV", scale: 2 },
];

pub const fn sensor(id: u16) -> Option<&'static SensorInfo> {
    match id {
        1 => Some(&SENSORS[0]),
        2 => Some(&SENSORS[1]),
        7 => Some(&SENSORS[2]),
        12 => Some(&SENSORS[3]),
        _ => None,
    }
}
```

и подключается макросом `include!`, который вставляет текст файла как
код Rust:

```rust
{{#include ../../code/ch08/src/generated.rs:include}}
```

Ошибка в CSV становится ошибкой сборки с номером строки, а не странным
поведением прошивки: сценарий паникует, и Cargo показывает его
сообщение. Тест `table_matches_csv` разбирает тот же CSV независимо от
`build.rs` и сверяет таблицу.

## Когда сценарий перезапускается

Без `rerun-if-changed` Cargo перезапускает сценарий при изменении любого
файла пакета. С ним — только при изменении перечисленных файлов. Это
видно в подробном выводе:

```console
$ touch code/ch08/src/lib.rs && cargo build -v -p ch08
       Dirty ch08 v0.1.0 (/home/user/rust-book/code/ch08): the file `code/ch08/src/lib.rs` has changed …
     Running `rustc --crate-name ch08 --edition=2024 code/ch08/src/lib.rs …`
$ touch code/ch08/sensors.csv && cargo build -v -p ch08
       Dirty ch08 v0.1.0 (/home/user/rust-book/code/ch08): the file `code/ch08/sensors.csv` has changed …
     Running `/home/user/rust-book/target/debug/build/ch08-96c918341999bb05/build-script-build`
     Running `rustc --crate-name ch08 --edition=2024 code/ch08/src/lib.rs …`
```

В первом случае сценарий не запускался, во втором — запустился до
компилятора. Строка `rerun-if-changed=build.rs` нужна, потому что
первая же строка `rerun-if-changed` отключает поведение по умолчанию, и
без неё правка самого сценария не привела бы к его перезапуску.

## Хост и цель

Сценарий — программа для машины, на которой идёт сборка. Когда крейт
собирается под Cortex-M, `build.rs` всё равно компилируется для x86_64 и
там же выполняется. Поэтому `cfg!` внутри сценария описывает хост, а
сведения о цели приходят в переменных `CARGO_CFG_*`. Второй файл,
который пишет `code/ch08/build.rs`, это фиксирует. Прошивка
`ch08-sections` печатает его содержимое:

```console
$ cd firmware && cargo run --release -q -p ch08-sections --bin target
ch08_bump (LDREX/STREX): было 41, стало 42
build.rs работал на x86_64, cfg!(target_arch = "arm") = false
крейт собран для arm
датчик  1 board_temp ×10 mC
датчик  2 ext_temp   ×10 mC
датчик  7 pressure   ×25 Pa
датчик 12 battery    ×2 mV
sensor(7) по адресу 0x16a0
```

Таблица, построенная на x86_64, работает на ARM: сценарий пишет
исходный текст, а не байты, и раскладку `SensorInfo` для цели выбирает
компилятор. Адрес 0x16a0 лежит во flash, это разобрано в разделе
[8.8](08-sections.md).

<div class="box trap">

**Ловушка.** `#[cfg(target_arch = "arm")]` в `build.rs` не сработает
никогда, если вы не собираете на ARM-машине: сценарий собран для хоста.
То же с `std::env::consts::ARCH`, `size_of::<usize>()` и порядком байтов.
Правильный вопрос к сценарию — `env::var("CARGO_CFG_TARGET_ARCH")`,
`CARGO_CFG_TARGET_POINTER_WIDTH`, `CARGO_CFG_TARGET_ENDIAN`. Тест
`build_script_runs_on_host` проверяет, что при сборке для хоста оба
ответа совпадают.

</div>

## Скрипт компоновки

Второе частое применение `build.rs` во встраиваемом коде — положить
описание памяти рядом с крейтом `cortex-m-rt`. Его скрипт компоновки
`link.x` подключает файл `memory.x`, который должен лежать там, где
компоновщик ищет файлы. Прошивки книги делят одну карту памяти, и каждая
копирует её так:

```rust
{{#include ../../firmware/ch08-sections/build.rs}}
```

Строка `rustc-link-search` добавляет `OUT_DIR` в пути поиска
компоновщика, а `rerun-if-changed` указывает на общий файл, который
лежит вне пакета. Устройство самой карты — в разделе
[8.8](08-sections.md).

## Данные без генерации

Если данные нужны как есть, генерировать ничего не надо: `include_str!`
и `include_bytes!` вшивают содержимое файла в программу при компиляции.
Калибровка датчика давления хранится в двоичном файле из двух чисел
little-endian:

```console
$ od -A d -t x1 code/ch08/calib.bin
0000000 10 27 00 00 9c ff ff ff
0000008
```

```rust
{{#include ../../code/ch08/src/generated.rs:calib}}
```

`include_bytes!` даёт `&'static [u8; N]` с выравниванием 1: компилятор
положит массив туда, где ему удобно, без гарантии, что адрес кратен 4.
Поэтому числа собираются из байтов через `from_le_bytes`, а не
приведением указателя к `*const u32`. Приведение скомпилировалось бы, но
чтение по невыровненному адресу на Cortex-M0 — исключение HardFault, а
в Rust — неопределённое поведение на любой платформе (глава 6, раздел
[6.7](../ch06/07-packed.md)). Тест `calib_from_file` проверяет и
значения, и выравнивание.

<div class="box trick">

**Трюк.** Если выравнивание всё-таки нужно (например, таблица
передаётся в DMA, который требует кратности 4), массив заворачивают в
структуру с повышенным выравниванием и копируют в неё содержимое файла
при компиляции:

```rust,ignore
#[repr(C, align(4))]
pub struct Aligned<T>(pub T);

pub static CALIB: Aligned<[u8; 8]> = Aligned(*include_bytes!("calib.bin"));
```

Размер файла компилятор проверит сам: с `[u8; 9]` та же строка даёт
E0308, несовпадение типов.

</div>

<div class="box check">

**Проверь себя.** Сценарий сборки читает `sensors.csv`, но строки
`rerun-if-changed` в нём нет. Что произойдёт при правке
`src/symbols.rs`? А если строка есть, но вы отредактировали
`sensors.csv` в другой ветке git и переключились на неё?

</div>
