# 29.3. Модуль, параметры и ошибки

## Минимальный модуль

```rust,ignore
{{#include ../../code/ch29/modules/ch29_hello.rs:all}}
```

Модуль — тип, а не пара функций. `module!` объявляет его и всё, что
загрузчику нужно знать о модуле; `kernel::Module::init` создаёт значение
этого типа при загрузке, а `Drop` — то, что в C было бы функцией
`module_exit`. Между ними значение живёт в статической памяти модуля, и
всё, что модуль держит, — поля этого значения.

Из сигнатуры видно три правила. `init` возвращает `Result<Self>`: отказ —
`Err` с кодом errno, и тогда модуль не загрузится, а значения не будет —
и `Drop` не вызовется. `ThisModule` — то, что в C называется
`THIS_MODULE`, ссылка на `struct module` для регистраций, которым нужен
владелец. И сам трейт требует `Sync + Send`: значение модуля доступно из
любого потока ядра, кто бы его ни вызвал.

Сеанс в QEMU (тест `hello_module_with_parameter`):

```text
# insmod /lib/ch29_hello.ko
warn:   ch29_hello: loading out-of-tree module taints kernel.
info:   ch29_hello: привет #1
# grep ch29 /proc/modules
ch29_hello 12288 0 - Live 0xffffffffa0000000 (O)
# rmmod ch29_hello
info:   ch29_hello: пока, было приветствий: 1
# insmod /lib/ch29_hello.ko times=3
info:   ch29_hello: привет #1
info:   ch29_hello: привет #2
info:   ch29_hello: привет #3
# rmmod ch29_hello
info:   ch29_hello: пока, было приветствий: 3
```

`pr_info!` — печать в журнал ядра с уровнем `KERN_INFO`, форматирование
— `core::fmt`, как у `println!`. Префикс `ch29_hello: ` макрос добавляет
сам — в C это делает `#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt` в
начале файла. А перевод строки — нет: журнал ядра делит записи по
`\n`, и без него следующая печать продолжит ту же строку.

Первая загрузка любого внешнего модуля «портит» (taints) ядро: в
`/proc/modules` у модуля флаг `(O)`, в каждом отчёте об ошибке —
`Tainted: G O`. Это не наказание, а подсказка тем, кто будет читать
отчёт: в ядре был код, которого нет в дереве исходников. Модуль лёг по
адресу `0xffffffffa0000000` — это начало области модулей x86-64
(раздел [29.4](04-memory.md)); 12288 — его размер в памяти, три
страницы.

## Параметры

Параметр модуля — переменная, значение которой задают при загрузке:
`insmod ch29_hello.ko times=3`. В `module!` у параметра тип, значение по
умолчанию и описание; в коде — `module_parameters::times.value()`.
Ошибки разбора ловит ядро, до вызова `init`:

```text
# insmod /lib/ch29_hello.ko times=11
insmod: Invalid argument (os error 22)
err:    ch29_hello: times = 11: не больше 10
# insmod /lib/ch29_hello.ko times=abc
insmod: Invalid argument (os error 22)
err:    ch29_hello: `abc' invalid for parameter `times'
# insmod /lib/ch29_hello.ko times=-1
insmod: Invalid argument (os error 22)
err:    ch29_hello: `-1' invalid for parameter `times'
# insmod /lib/ch29_hello.ko count=2
warn:   ch29_hello: unknown parameter 'count' ignored
info:   ch29_hello: привет #1
```

Первую ошибку вернул `init` модуля — `Err(EINVAL)`, и `insmod` получил
её как `EINVAL` от `finit_module`. Вторую и третью — разбор параметра:
`u32` не принимает ни букв, ни минуса. Четвёртая — не ошибка: ядро
пропускает неизвестный параметр с предупреждением, и модуль
загрузился со значением по умолчанию. Опечатку в имени параметра
никто не остановит.

Всё, что `module!` знает о модуле, попадает в секцию `.modinfo`:

```console
$ readelf -p .modinfo ch29_hello.ko

String dump of section '.modinfo':
  [     0]  parmtype=times:u32
  [    13]  parm=times:How many times to say hello
  [    3a]  author=Rust textbook
  [    4f]  description=Minimal module with a parameter
  [    7b]  license=GPL
  [    87]  name=ch29_hello
  [    97]  depends=
  [    a0]  vermagic=7.0.0 SMP preempt mod_unload
```

<div class="box c">

**Как в C.** В C модуль — две функции, `module_init` и `module_exit`, и
глобальные переменные; параметр — переменная плюс `module_param`. Вот
параметр модуля `ch29_errors_c` из следующего подраздела:

```c
{{#include ../../code/ch29/modules/ch29_errors_c.c:param}}
```

Тип в C пишут дважды — у переменной и именем из списка ядра (`uint`,
`charp`, `bool`) в `module_param`, — и макрос проверяет, что они
согласны. В Rust тип один, и `.modinfo` называет его по-своему:
`parmtype=times:u32` против `parmtype=fail_at:uint`. Последний аргумент
`module_param` — права файла параметра в
`/sys/module/<модуль>/parameters/`. В 7.0 параметры модулей на Rust —
только целые числа, а права — 0: в sysfs их не видно, и после загрузки
их не поменять.

</div>

## Ошибки как значения

Ошибка в ядре — отрицательное число errno: `-ENOMEM`, `-EINVAL`. В крейте
`kernel` это тип `Error` — обёртка над `NonZeroI32` с инвариантом
«значение в диапазоне errno», — и `Result<T>` с ним по умолчанию.
Константы — `ENOMEM`, `EINVAL` и остальные — в прелюдии. На границе с C
абстракции переводят `Error` в число (`to_errno`) и обратно
(`from_errno`, `to_result`), и `Err(EINVAL)` из `init` уходит
пользователю как `EINVAL` от `finit_module`.

Чтобы увидеть, как это работает на пути ошибки, модулю нужно несколько
ресурсов. `ch29_errors` держит три: буфер в куче, misc-устройство
`/dev/ch29-errors` и каталог в debugfs. Параметр `fail_at` имитирует
отказ на шаге 1–4. Каждый ресурс обёрнут в `Logged`, который сообщает в
журнал, когда его освобождают:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_errors.rs:logged}}
```

И `init`:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_errors.rs:init}}
```

Модуль — `InPlaceModule`: регистрация misc-устройства должна лежать на
месте, и значение модуля инициализируется сразу в статической памяти
(раздел [29.5](05-pin-init.md) объясняет, почему). Макрос
`try_pin_init!` инициализирует поля по очереди, и в каждом можно писать
`?`. Когда выражение поля возвращает ошибку, макрос освобождает уже
готовые поля — в обратном порядке — и возвращает ошибку дальше. Ни
одной строки отката в модуле нет:

```text
# insmod /lib/ch29_errors.ko fail_at=1
insmod: Cannot allocate memory (os error 12)
info:   ch29_errors: шаг 1: отказ ENOMEM
# ls -d /dev/ch29-errors: нет
# insmod /lib/ch29_errors.ko fail_at=2
insmod: Device or resource busy (os error 16)
info:   ch29_errors: шаг 2: отказ EBUSY
info:   ch29_errors: освобождаю: буфер
# ls -d /dev/ch29-errors: нет
# insmod /lib/ch29_errors.ko fail_at=3
insmod: No such file or directory (os error 2)
info:   ch29_errors: шаг 3: отказ ENOENT
info:   ch29_errors: освобождаю: /dev/ch29-errors
info:   ch29_errors: освобождаю: буфер
# ls -d /dev/ch29-errors: нет
# insmod /lib/ch29_errors.ko fail_at=4
insmod: Invalid argument (os error 22)
info:   ch29_errors: шаг 4: отказ EINVAL
info:   ch29_errors: освобождаю: каталог debugfs
info:   ch29_errors: освобождаю: /dev/ch29-errors
info:   ch29_errors: освобождаю: буфер
# ls -d /dev/ch29-errors: нет
```

`{:?}` у `Error` печатает имя кода — `ENOMEM`, а не `-12`. Имена берутся
из таблицы ядра `errname()`, которая есть только с
`CONFIG_SYMBOLIC_ERRNAME`; без неё печать была бы `Error(12)`.

<div class="box c">

**Как в C.** Тот же модуль на C — `ch29_errors_c.c`, классическая
цепочка `goto`:

```c
{{#include ../../code/ch29/modules/ch29_errors_c.c:init}}
```

Сеанс с `fail_at=1…4` — тот же, строка в строку, кроме кода в
сообщении (`отказ -12`): тест `errors_roll_back_in_reverse_order`
сверяет оба модуля одними и теми же ожиданиями. Разница — в том, кто
отвечает за порядок. В C порядок меток — договорённость, которую
проверяет только чтение кода: переставьте `err_misc` и `err_buf`, и
модуль соберётся без предупреждений. `return ret` вместо `goto err_buf`
после неудачного `misc_register` — утечка буфера — тоже. В Rust
порядок отката выводится из порядка инициализации, и ошибиться в нём
негде.

</div>

## Выгрузка: порядок полей

Успешная загрузка и выгрузка:

```text
# insmod /lib/ch29_errors.ko
info:   ch29_errors: загружен: буфер, /dev/ch29-errors, debugfs
# ls -d /dev/ch29-errors: есть
# ls -d /sys/kernel/debug/ch29_errors: есть
# rmmod ch29_errors
info:   ch29_errors: освобождаю: буфер
info:   ch29_errors: освобождаю: /dev/ch29-errors
info:   ch29_errors: освобождаю: каталог debugfs
```

А у модуля на C:

```text
# rmmod ch29_errors_c
info:   ch29_errors_c: освобождаю: каталог debugfs
info:   ch29_errors_c: освобождаю: /dev/ch29-errors-c
info:   ch29_errors_c: освобождаю: буфер
```

<div class="box trap">

**Ловушка: при выгрузке поля освобождаются в порядке объявления.** Откат
в `try_pin_init!` идёт в обратном порядке, а `Drop` структуры — в прямом:
сначала первое объявленное поле. Это правило языка, и для модуля оно
значит, что буфер освободится раньше, чем снимется устройство,
которое — в настоящем драйвере — могло бы им пользоваться. В этом
модуле устройство буфера не касается, и порядок безразличен. Если
ресурсы зависят друг от друга, объявляйте поля в порядке, обратном
зависимостям (сначала то, что пользуется, потом то, чем пользуются), или
снимайте зависимые ресурсы явно в своём `Drop`. Тест сверяет порядок
выгрузки для обоих модулей.

</div>

<div class="box check">

**Проверь себя.**

1. Что увидит пользователь, если `init` вернёт `Err(ENOSPC)`? А если
   модуль напишет `return Err(Error::from_errno(-5000))`?
2. Почему у `ch29_hello` нет `InPlaceModule`, а у `ch29_errors` есть?
3. Переставьте в `Errors` поля `buf` и `dir`. Что изменится в выводе
   сеанса с `fail_at=4` и при выгрузке?

</div>
