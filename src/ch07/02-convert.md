# 7.2. Преобразование ошибок и комбинаторы

## ? вызывает From

Драйвер датчика проверяет не только обмен по шине, но и диапазон
значения. У него свой тип ошибки, и ошибка шины — один из вариантов:

```rust
{{#include ../../code/ch07/src/convert.rs:sensor_error}}
```

`read_temp(bus)?` возвращает `Result<i16, BusError>`, а функция —
`Result<i16, SensorError>`. Раскрытие `?` из раздела
[7.1](01-result.md) вызывает `From::from(e)`, и ровно эта реализация
превращает `BusError` в `SensorError::Bus`. Без неё компилятор ответит:

```text
error[E0277]: `?` couldn't convert the error to `SensorError`
```

Так устроены слои: каждый уровень программы объявляет свой тип ошибки
и реализует `From` для ошибок уровня ниже. Код с `?` при этом не
меняется.

<div class="box trap">

**Ловушка.** `From` не транзитивен. Если есть `From<io::Error> for
ConfigError` и `From<ConfigError> for AppError`, то `?` на
`io::Error` в функции, которая возвращает `AppError`, не скомпилируется:
компилятор ищет ровно `From<io::Error> for AppError` и цепочку из двух
преобразований сам не строит. Это одна из ловушек задачи
[7.A](09-tasks.md).

</div>

## Box&lt;dyn Error&gt;: любая ошибка

Когда вызывающему не нужно различать причины (прототип, `main`,
утилита), подойдёт «любая ошибка»:

```rust
{{#include ../../code/ch07/src/convert.rs:boxed}}
```

Здесь работают две реализации из `alloc`: `From<E> for Box<dyn Error>`
для любого `E: Error` и `From<&str> for Box<dyn Error>` (строка
оборачивается в скрытый тип ошибки). С `+ Send + Sync` ошибку можно
передать в другой поток (глава 20). Вызывающий теряет возможность
сопоставить варианты `match`-ем, но может спросить у ошибки её
настоящий тип:

```rust,ignore
let e = parse_port("80x").unwrap_err();
assert!(e.downcast_ref::<core::num::ParseIntError>().is_some());
```

`downcast_ref` сравнивает `TypeId` типа внутри `Box` с запрошенным:
это проверка во время выполнения, аналог `dynamic_cast` из C++.

<div class="box ref">

**Справка: `Error` без `std`.** Трейт `Error` живёт в `core::error` с
Rust 1.81, а реализации `From` для `Box<dyn Error>` — в `alloc`. Поэтому
весь этот раздел работает и в прошивке с [глобальным аллокатором][g-global-allocator], и в
[`no_std`][g-no-std] без него, если не использовать `Box`. [Крейт][g-crate] `code/ch07`
собирается для трёх [целей][g-target] Cortex-M с `--no-default-features`.

</div>

## Комбинаторы вместо вложенных match

`Option` и `Result` несут набор методов, которые преобразуют значение,
не разбирая его вручную. Такие методы называют *комбинаторами*
(combinators): из них, как из звеньев, собирается цепочка шагов. Поиск
периода опроса датчика по таблице:

```rust
{{#include ../../code/ch07/src/combinators.rs:lookup}}
```

Обе функции дают одинаковый ответ для каждого `id` (тест в том же
файле). Самые нужные методы:

| Метод | Что делает |
|---|---|
| `ok_or(e)`, `ok_or_else(f)` | `Option` → `Result`: `None` становится `Err(e)` |
| `ok()`, `err()` | `Result` → `Option`, другая половина выбрасывается |
| `map(f)`, `map_err(f)` | преобразовать значение или ошибку |
| `and_then(f)` | следующий шаг, который тоже может не получиться |
| `filter(p)` | `Some(x)` → `None`, если `x` не годится |
| `unwrap_or(d)`, `unwrap_or_default()` | значение по умолчанию вместо ошибки |
| `transpose()` | `Option<Result<T, E>>` ↔ `Result<Option<T>, E>` |

Аргументы `ok_or` вычисляются всегда, даже если `Option` — `Some`. Если
ошибка дорогая (строка, `format!`), берите `ok_or_else` с [замыканием][g-closure].
Замыкания подробно разбираются в главе 12.

`transpose` нужен для необязательных полей: отсутствие поля — не
ошибка, а неверное значение — ошибка:

```rust
{{#include ../../code/ch07/src/combinators.rs:transpose}}
```

## Infallible: ошибка, которой не бывает

```rust
{{#include ../../code/ch07/src/convert.rs:infallible}}
```

`TryFrom<u8> for u16` реализован через `From`, и его тип ошибки —
`core::convert::Infallible`, перечисление без вариантов. Значения такого
типа не существует, поэтому `Err` невозможен, и с Rust 1.82 [образец][g-pattern]
`Ok(w)` считается *неопровержимым* (irrefutable pattern): `let Ok(w) = …;`
компилируется без `else`. Размер `Result<u32, Infallible>` равен размеру
`u32`: компилятор знает, что признак варианта хранить незачем.

`Infallible` встречается в [обобщённом][g-generic] коде: трейт требует тип ошибки, а
конкретная реализация ошибиться не может. Например, трейты
[`embedded-hal`][g-embedded-hal] для вывода GPIO требуют тип ошибки, а [HAL][g-hal]-крейты
микроконтроллеров, где запись в регистр не может не получиться,
объявляют `type Error = Infallible` (раздел [27.5](../ch27/05-hal.md)).

## #[must_use]: выброшенный результат

`Result` помечен `#[must_use]`. Вызов, результат которого никуда не
делся, даёт предупреждение `unused_must_use`, а в проектах с
`-D warnings` (как в CI этой книги) — ошибку. Свою функцию можно
пометить так же, с объяснением:

```rust
{{#include ../../code/ch07/src/convert.rs:must_use}}
```

Doc-тесты в `code/ch07/src/convert.rs` проверяют оба случая. Явный
отказ от значения пишется `let _ = send(b"ping");`, и это видно на
ревью, в отличие от молчаливого `(void)` в C, которое GCC, как показал
`unchecked.c`, даже не принимает.

<div class="box trap">

**Ловушка.** `let _ = mutex.lock();` не держит замок: `_` не связывает
значение, и [страж][g-guard] уничтожается сразу (раздел
[3.6](../ch03/06-guards.md)). А `let _guard = mutex.lock();` держит
замок до конца блока. Одна буква в имени меняет [время жизни][g-lifetime] значения.

</div>

<div class="box check">

**Проверь себя.**

1. Почему `parse_port("0")` не находит `ParseIntError` через
   `downcast_ref`, хотя тоже возвращает ошибку?
2. Что выгоднее в горячем цикле: `opt.ok_or(format!(…))` или
   `opt.ok_or_else(|| format!(…))`, и почему?
3. Зачем трейту нужен тип ошибки `Infallible`, если можно было бы
   просто не возвращать `Result`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-closure]: ../appendix-f.md#closure "Замыкание (closure): Функция вместе со своим контекстом — захваченными переменными."
[g-crate]: ../appendix-f.md#crate "Крейт (crate): Единица компиляции Rust: одно дерево модулей, один вызов rustc, один результат — библиотека или программа."
[g-embedded-hal]: ../appendix-f.md#embedded-hal "embedded-hal: Крейт трейтов периферии микроконтроллера: OutputPin, InputPin, DelayNs, SpiDevice, I2c."
[g-generic]: ../appendix-f.md#generic "Обобщение (generic): Функция или тип с параметрами-типами: fn max(a: T, b: T) -> T."
[g-global-allocator]: ../appendix-f.md#global-allocator "Глобальный аллокатор (global allocator): Распределитель памяти, который зовут Box, Vec, String и все коллекции alloc: тип, реализующий GlobalAlloc и объявленный атрибутом #[global_allocator]."
[g-guard]: ../appendix-f.md#guard "Страж (guard): Значение, чей деструктор отпускает ресурс: MutexGuard отпускает мьютекс, IrqGuard из главы 3 снова разрешает прерывания."
[g-hal]: ../appendix-f.md#hal "HAL (hardware abstraction layer): Уровень абстракции оборудования — библиотека над регистрами кристалла: вместо регистров — выводы, UART, таймеры."
[g-lifetime]: ../appendix-f.md#lifetime "Время жизни (lifetime): Имя для участка программы, на котором ссылка действительна: 'a в &'a str."
[g-no-std]: ../appendix-f.md#no-std "no_std: Атрибут крейта #![no_std]: крейт пользуется только core (и, если нужно, alloc), без стандартной библиотеки ОС."
[g-pattern]: ../appendix-f.md#pattern "Образец (pattern): Описание формы значения в match, let, if let и параметрах функций: литералы, диапазоны, альтернативы |, разбор кортежей, структур и срезов, привязки имён."
[g-target]: ../appendix-f.md#target "Цель (target): То, для чего генерируется код: архитектура, ОС и ABI, записанные в имени-триплете (thumbv7m-none-eabi)."
