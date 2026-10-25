# 24.7. Задачи

Все задачи решаются кодом. Заготовки — в `exercises/ch24` (применения,
часы, вызовы макросов) и `exercises/ch24-macros` (сами макросы); тесты,
общие для заготовок и решений, — в `tests-shared/ch24.rs`, снимки ошибок
— в `tests-shared/ch24-ui`. Крейт макросов подключён под одним именем
`tasks_macros` и у заготовок, и у решений, поэтому одни и те же снимки
trybuild годятся для обоих.

```console
$ cargo test -p ch24-exercises            # все задачи
$ cargo test -p ch24-exercises b_         # одна задача (и rustc_b_errors для снимков B)
$ cargo test -p ch24-solutions            # эталон
```

Заготовки макросов уже разбирают вход и порождают все нужные имена, но с
заглушками внутри: тесты компилируются и падают, пока задача не решена.
Разборы — в [приложении E](../appendix-e/ch24.md), но сначала попробуйте
сами.

## A. Два раскрытия

В прошивке `firmware/ch24-expand` работают два процедурных макроса —
встроенный `#[derive(Debug)]` и `#[entry]` из cortex-m-rt:

```rust,ignore
{{#include ../../firmware/ch24-expand/src/main.rs:types}}
```

```rust,ignore
{{#include ../../firmware/ch24-expand/src/main.rs:main}}
```

```console
$ cd firmware && cargo run --release -p ch24-expand
Point { x: 3, y: -4 } Off Blink(2)
BOOTS = 1
```

Без компилятора предскажите и впишите в `predicted` четыре фрагмента
раскрытия: `impl Debug` для `Point`, `impl Debug` для `Mode`, функцию,
которую `#[entry]` экспортирует под символом `main`, и начало функции
пользователя — сигнатуру и первый оператор:

```rust,ignore
{{#include ../../exercises/ch24/src/a_expand.rs:skeleton}}
```

Затем сверьте с компилятором:

```console
$ cd firmware
$ RUSTC_BOOTSTRAP=1 cargo rustc -p ch24-expand --profile check -- -Zunpretty=expanded
```

или `cargo expand`. Тест `rustc_a_expansions_match_prediction` запускает
ту же команду и сравнивает без учёта пробелов, переводов строк и
строк-комментариев. Если не совпало, сначала поймите, *почему*: в
раскрытии `Debug` есть по меньшей мере две вещи, которых человек руками не
напишет, а `#[entry]` объясняет, откуда в `main` взялся `&'static mut`
без единого `unsafe` у пользователя. Каждую строку раскрытия нужно
уметь объяснить — разбор в приложении E идёт построчно.

## B. Битовые поля регистра

`#[derive(Bitfield)]` упаковывает поля с `#[bits(N)]` в слово `u32`.
Пример — формат кадра PL011, регистр UARTLCRH:

```rust,ignore
{{#include ../../solutions/ch24/src/b_bitfield.rs:lcrh}}
```

Derive не может изменить структуру, поэтому рядом с ней появляется тип
`LcrhBits(pub u32)` — само слово регистра. Требования тестов `b_*`:

- поля занимают биты подряд с нулевого, в порядке объявления:
  `LcrhBits::WLEN_SHIFT == 5`, `WLEN_MASK == 0x60`, `WIDTH == 8`;
- на каждое поле — константы `X_SHIFT` и `X_MASK` и три `const fn`:
  `x(self)`, `set_x(&mut self, v)` и `with_x(self, v) -> Self`. Методы —
  `const`, потому что ими собирают константы:
  `LCRH_8N1_FIFO = LcrhBits(0).with_wlen(3).with_fen(true)`;
- `set_*` с не помещающимся значением паникует:
  «Lcrh.wlen: значение не помещается в 2 бит»;
- `From` в обе стороны и `Debug`, который печатает поля:
  `LcrhBits { brk: false, pen: false, eps: false, stp2: false, fen: true,
  wlen: 3, sps: false }`;
- поле во всё слово (`#[bits(32)] all: u32`) работает, и маска — `u32::MAX`
  без переполнения сдвига.

Типы полей — `bool` (ровно 1 бит), `u8`, `u16` и `u32` (от 1 бита до
ширины типа). Всё остальное — ошибка компиляции *со спаном*: поле без
`#[bits]` — на имени поля, не тот тип — на типе, неверная ширина — на
литерале ширины, выход за 32 бита — на литерале того поля, которое
вышло за край:

```text
{{#include ../../tests-shared/ch24-ui/b_overflow.stderr}}
```

Все ошибки структуры — за одну сборку (снимки `b_bad_type`,
`b_too_wide`). Заготовка разбирает поля и порождает все имена, но с
нулями и пустыми телами:

```rust,ignore
{{#include ../../exercises/ch24-macros/src/bitfield.rs:skeleton}}
```

## C. Замер времени функции

`#[timed(STATS)]` на функции считает её вызовы, суммарное и наибольшее
время одного вызова. Часы — параметр типа:

```rust,ignore
{{#include ../../exercises/ch24/src/c_timed.rs:clock}}
```

Счётчик и [охранник][g-guard] даны:

```rust,ignore
{{#include ../../exercises/ch24/src/c_timed.rs:timing}}
```

Часы для Cortex-M тоже даны — [SysTick][g-systick] на любом ядре и счётчик тактов
[DWT][g-dwt] на [ARMv7-M][g-armv7m] и новее:

```rust,ignore
{{#include ../../exercises/ch24/src/c_timed.rs:systick}}
```

```rust,ignore
{{#include ../../exercises/ch24/src/c_timed.rs:dwt}}
```

На хосте — `HostClock` (наносекунды от `Instant`) и `FakeClock`, у
которого время идёт, только когда его двигают. Заготовка макроса
возвращает функцию без изменений:

```rust,ignore
{{#include ../../exercises/ch24-macros/src/timed.rs:skeleton}}
```

Требования тестов `c_*`:

- считается каждый выход: обычный возврат, `return`, `?` и паника с
  [раскруткой стека][g-unwinding] (`c_every_exit_is_counted`, `c_panic_is_counted`);
- аргументы функции с именами `guard`, `_guard`, `__guard`, `start` и
  `timing` работают как прежде (`c_arguments_keep_their_names`);
- атрибут работает на методах и обобщённых функциях;
- без аргумента, с аргументом не-путём, на `const fn` и на `async fn` —
  ошибки компиляции со спанами (снимки `c_*`); для `async fn` объясните в
  тексте ошибки, *почему* нельзя.

Прошивка `firmware/ch24-tasks` замеряет функцию передачи по SysTick:

```rust,ignore
{{#include ../../firmware/ch24-tasks/src/main.rs:timed}}
```

`Timing` копит время через `fetch_add` и `fetch_max`, которых на
ARMv6-M нет, поэтому на Cortex-M0 замер выключен `cfg_attr` — атрибутный
макрос под условием работает как любой другой атрибут. В QEMU счётчик
DWT не моделируется: `DwtCycles::enable()` возвращает `false`, а SysTick
идёт по часам хоста, и число тактов меняется от запуска к запуску —
`firmware/run-demos.sh` сверяет только число вызовов.

## D. Регистры из файла SVD

Функциональный макрос `include_svd!("файл.svd", "UART0")` читает файл
CMSIS-SVD при компиляции и порождает модуль `uart0` с API регистров одной
периферии. Файл главы — `code/ch24/svd/lm3s6965.svd`, два UART
LM3S6965. Регистр в нём:

```text
{{#include ../../code/ch24/svd/lm3s6965.svd:115:153}}
```

и второй UART, описанный через первый:

```text
{{#include ../../code/ch24/svd/lm3s6965.svd:487:494}}
```

Вызовы (путь — от каталога крейта, который вызывает макрос):

```rust,ignore
{{#include ../../exercises/ch24/src/d_svd.rs:invoke}}
```

Требования тестов `d_*`:

- `BASE`; `RegisterBlock` — `#[repr(C)]` с промежутками там, где
  регистров нет: `offset_of!(RegisterBlock, fr) == 0x18`, размер 0x48;
- тип на каждый регистр (`Fr`, `Lcrh`…) с константами `OFFSET`, `RESET`
  (`Ctl::RESET == 0x300`) и `Field` на каждое поле:
  `Lcrh::WLEN.mask() == 0x60`, `get`, `set`;
- права доступа — наличием методов: у `read-only` нет `write`, у
  `write-only` нет `read`, `modify` есть только у `read-write` (снимок
  `d_access`, E0599);
- `size` 8, 16 или 32 — тип регистра `u8`, `u16` или `u32`;
  `derivedFrom` — регистры образца по своему адресу;
- `steal()` по адресу и `RegisterBlock::from_ptr` для блока в обычной
  памяти — на нём тест `d_access_on_memory` проверяет чтение и запись;
- ошибки — со спанами: нет файла — на литерале пути, нет периферии — на
  её имени со списком тех, что есть, ошибка в файле — на литерале пути с
  `файл:строка:столбец` (снимки `d_no_file`, `d_no_peripheral`,
  `d_broken_svd`);
- файл — зависимость сборки: после его правки крейт пересобирается
  (`rustc_d_svd_is_build_dependency`, раздел [24.6](06-cost.md)).

Заготовка разбирает аргументы и файл (через крейт roxmltree), находит
периферию и порождает все имена, но блок без промежутков, `OFFSET` и
`RESET` — нули, методы паникуют, а любая ошибка — паника макроса:

```rust,ignore
{{#include ../../exercises/ch24-macros/src/svd.rs:skeleton}}
```

Проверка на железе — та же прошивка `firmware/ch24-tasks`, где UART0
описан вашим макросом, а формат кадра записан значением из задачи B:

```rust,ignore
{{#include ../../firmware/ch24-tasks/src/main.rs:svd}}
```

```rust,ignore
{{#include ../../firmware/ch24-tasks/src/main.rs:main}}
```

```console
$ cd firmware && cargo run --release -p ch24-tasks
CTL: 0x300, по SVD 0x300
IFLS: 0x12, по SVD 0x12
LCRH = Lcrh { brk: false, pen: false, eps: false, stp2: false, fen: true, wlen: 3, sps: false }
LCRH.WLEN по SVD: 3
uart0: hello from include_svd!
uart0: SVD -> RegisterBlock
uart0: #[timed]
send: вызовов 3, самый долгий не дольше всех: true
send: 34582 тактов SysTick, самый долгий 14938
DWT.CYCCNT включился: false
```

Значения после сброса из файла совпали с тем, что QEMU отдаёт для PL011;
`LCRH`, записанный константой задачи B, прочитан обратно и разобран и её
`From`, и полем `WLEN` задачи D.

Вторая часть задачи — сравнение с **svd2rust**, генератором, на котором
построены настоящие крейты устройств (PAC). Поставьте его (`cargo
install svd2rust`), сгенерируйте крейт из того же файла и сравните с
`include_svd!`: сколько кода порождает каждый, сколько длится чистая
сборка и пересборка, сколько крейтов тянет каждый и какой машинный код
получается для одной и той же записи в регистр. Что из API svd2rust ваш
макрос не умеет и что из этого стоило бы добавить?

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-armv7m]: ../appendix-f.md#armv7m "ARMv6-M, ARMv7-M, ARMv7E-M: Версии архитектуры ARM для микроконтроллеров."
[g-dwt]: ../appendix-f.md#dwt "DWT (Data Watchpoint and Trace): Отладочный блок Cortex-M3, M4, M7 и M33 с точками наблюдения и счётчиками, среди них — 32-битный счётчик тактов CYCCNT по адресу 0xE000_1004."
[g-guard]: ../appendix-f.md#guard "Страж (guard): Значение, чей деструктор отпускает ресурс: MutexGuard отпускает мьютекс, IrqGuard из главы 3 снова разрешает прерывания."
[g-systick]: ../appendix-f.md#systick "SysTick: Системный таймер Cortex-M: 24-битный счётчик, который считает вниз от заданного значения и при переходе через ноль вызывает исключение SysTick."
[g-unwinding]: ../appendix-f.md#unwinding "Раскрутка стека (unwinding): Снятие кадров стека по одному после паники с вызовом деструкторов живых значений."
