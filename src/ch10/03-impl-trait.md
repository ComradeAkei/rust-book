# 10.3. `impl Trait` в аргументе и в результате

## Безымянный параметр

CRC-8 нужен по массиву байтов, по срезу, по итератору, который читает
FIFO, по строке. Писать `<I: IntoIterator<Item = u8>>` ради одного
аргумента длинно, и для этого есть сокращение:

```rust
{{#include ../../code/ch10/src/impl_trait.rs:arg}}
```

`impl IntoIterator<Item = u8>` в позиции аргумента — тот же параметр
типа, только без имени. Код получается тот же: своя копия `crc8` на
каждый тип аргумента, и тест вызывает её с `[u8; 9]`, `Copied<Iter<u8>>`,
`Vec<u8>` и `Bytes` — четыре копии с одинаковым результатом 0xf4,
контрольным значением CRC-8 для строки `"123456789"`. Разница с
именованным параметром одна: безымянный нельзя задать явно.

```text
error[E0107]: function takes 0 generic arguments but 1 generic argument was supplied
 --> e0107.rs:2:23
  |
2 |     ch10::impl_trait::crc8::<[u8; 2]>([1, 2])
  |                       ^^^^----------- help: remove the unnecessary generics
  |                       |
  |                       expected 0 generic arguments
  |
  = note: `impl Trait` cannot be explicitly specified as a generic argument
```

Для библиотечной функции это часть интерфейса: если пользователи
привыкли писать turbofish, замена `<I: …>` на `impl …` сломает им сборку.

## Скрытый тип результата

В позиции результата `impl Trait` значит другое: функция возвращает
*один* конкретный тип, но вызывающему сообщает о нём только то, что
написано после `impl`.

```rust
{{#include ../../code/ch10/src/impl_trait.rs:ret}}
```

Настоящий тип результата `evens` — `StepBy<Range<u32>>`, и
`type_name_of_val` в тесте его показывает. Но вызывающий знает только,
что это итератор `u32`, который можно клонировать, и сохранить его в
переменную типа `Range<u32>` не может:

```text
error[E0308]: mismatched types
 --> e0308b.rs:2:36
  |
2 |     let n: core::ops::Range<u32> = ch10::impl_trait::evens(10);
  |            ---------------------   ^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Range<u32>`, found opaque type
  |            |
  |            expected due to this
  |
…
  = note:   expected struct `std::ops::Range<u32>`
          found opaque type `impl Iterator<Item = u32> + Clone`
```

Это и есть смысл: автор `evens` может заменить `step_by` на `filter`, и
ни один вызывающий не сломается. `+ Clone` — тоже часть обещания: без
него `e.clone()` в тесте не собрался бы, хотя `StepBy` клонировать
умеет.

У замыкания имени типа нет вовсе, и вернуть его можно только так:
`scaler(3300, 4095)` отдаёт функцию перевода отсчёта 12-битного АЦП в
милливольты. Замыкание захватило два `i32`, и его размер — 8 байт, как у
структуры из двух полей; указателя на код в нём нет, потому что код
определяется типом. Подробно замыкания разбирает глава 12.

Скрытый тип один на функцию. Две ветки с разными типами не пройдут:

```text
error[E0308]: `if` and `else` have incompatible types
 --> e0308a.rs:2:43
  |
2 |     if fast { (0..10).step_by(2) } else { (0..10).map(|x| x * 3) }
  |               ------------------          ^^^^^^^^^^^^^^^^^^^^^^ expected `StepBy<Range<{integer}>>`, found `Map<Range<{integer}>, {closure@...}>`
  |               |
  |               expected because of this
  |
…
help: you could change the return type to be a boxed trait object
  |
1 - pub fn samples(fast: bool) -> impl Iterator<Item = u32> {
1 + pub fn samples(fast: bool) -> Box<dyn Iterator<Item = u32>> {
  |
```

Компилятор предлагает `Box<dyn Iterator>`, то есть кучу и таблицу
методов (глава 11). Без кучи обходятся иначе: сводят ветки к одному
типу, вынося различие в данные, —

```rust
let (step, k) = if fast { (2, 1) } else { (1, 3) };
(0..10).step_by(step).map(move |x| x * k)
```

— или заводят перечисление из двух вариантов, которое само реализует
`Iterator`.

<div class="box hood">

**Под капотом.** Скрытый тип скрыт только от программиста. Компилятор
знает его точно, и вызов `evens(10).sum()` мономорфизуется для
`StepBy<Range<u32>>` и встраивается целиком, как если бы тип был
написан явно. `impl Trait` — статическая диспетчеризация: ни кучи, ни
таблицы, ни косвенных вызовов. Этим он отличается от `Box<dyn Trait>`,
который тоже прячет тип, но уже от компилятора.

</div>

## Что захватывает скрытый тип

Если функция принимает ссылку, скрытый тип в редакции 2024 считается
держащим её, даже когда на деле не держит:

```rust
{{#include ../../code/ch10/src/impl_trait.rs:capture}}
```

Итератор индексов — `Range<usize>`, ссылок в нём нет. Но по сигнатуре
`indices` он захватил время жизни `v`, и пока итератор жив, `v` менять
нельзя:

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> e0502.rs:6:5
  |
5 |     let it = indices(&v);
  |                      -- immutable borrow occurs here
6 |     v.push(4);
  |     ^^^^^^^^^ mutable borrow occurs here
7 |     assert_eq!(it.count(), 3);
  |                -- immutable borrow later used here
  |
note: this call may capture more lifetimes than intended, because Rust 2024 has adjusted the `impl Trait` lifetime capture rules
…
help: if you can modify this crate, add a precise capturing bound to avoid overcapturing: `+ use<>`
```

`+ use<>` в `indices_free` перечисляет захваченное явно, и пустой список
отпускает заём на выходе из функции. Обещать пустой список можно, только
если это правда: `v.iter().copied()` держит ссылку, и `use<>` для него
даёт E0700. Правила захвата и их отличие от редакции 2021 подробно
разобраны в разделе [5.7](../ch05/07-capture.md).

## `impl Trait` в методе трейта

С Rust 1.75 `impl Trait` можно вернуть и из метода трейта. Каждая
реализация тогда возвращает свой скрытый тип:

```rust
{{#include ../../code/ch10/src/impl_trait.rs:rpitit}}
```

`AdcBuf::samples` отдаёт `Copied<slice::Iter<i16>>`, `Saw::samples` —
`Map<Range<i16>, {closure}>`. Обобщённая `peak<S: Source>`
мономорфизуется для каждого источника, и в каждой копии тип итератора
известен точно. До 1.75 такой трейт пришлось бы писать с ассоциированным
типом `type Iter: Iterator<Item = i16>`, и для `Saw` этот тип было бы
нечем назвать: у замыкания нет имени.

<div class="box trap">

**Ловушка.** Скрытый тип нельзя назвать, и это мешает хранить его.
Структуре с полем «итератор из `evens`» нужен параметр типа,
`struct Gen<I: Iterator<Item = u32>> { it: I }`, потому что записать
`Gen { it: StepBy<…> }` нельзя, а именованные скрытые типы (`type Evens =
impl Iterator`) пока нестабильны. Трейт с `impl Trait` в методе к тому же
нельзя использовать как `dyn Source`: о совместимости с `dyn` речь в
главе 11.

</div>

<div class="box check">

**Проверь себя.** Сколько копий `crc8` создаст тест `argument_position`?
Почему `size_of_val(&scaler(3300, 4095))` равен 8, а не 16, хотя
замыкание «содержит код»? Что изменится для вызывающих, если в `evens`
убрать `+ Clone`, а что — если заменить `step_by(2)` на `filter(|x| x % 2
== 0)`?

</div>
