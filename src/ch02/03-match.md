# 2.3. match и образцы

## Полнота

`match` сравнивает значение с образцами сверху вниз и выполняет первую
подошедшую ветку. Ветки обязаны покрыть все возможные значения — это
требование *полноты* (exhaustiveness), и оно проверяется при компиляции. Функция `mode_name` из введения на Rust не
соберётся:

```text
error[E0004]: non-exhaustive patterns: `Mode::Sleep` not covered
  --> m2.rs:9:11
   |
 9 |     match m {
   |           ^ pattern `Mode::Sleep` not covered
   |
note: `Mode` defined here
  --> m2.rs:1:6
   |
 1 | enum Mode {
   |      ^^^^
...
 5 |     Sleep, // добавлен позже, чем написана mode_name
   |     ----- not covered
   = note: the matched value is of type `Mode`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
   |
12 ~         Mode::Tx => "tx",
13 ~         Mode::Sleep => todo!(),
   |
```

Тот же исходник лежит doc-тестом `compile_fail,E0004` в
`code/ch02/src/patterns.rs`. Полнота проверяется и для целых: `match` по
`u8` с ветками `0..=0x7F` и `0x80..=0xFF` полон и без `_`, а по `u32` с
теми же ветками — нет.

## Образцы

*Образец* (pattern) описывает форму значения. Литералы, диапазоны, альтернативы и
привязки комбинируются как угодно:

```rust
{{#include ../../code/ch02/src/patterns.rs:classify}}
```

- `b'0'..=b'9'` — диапазон с концом, `0..0x20` — без конца, `0x80..` —
  до максимума типа.
- `|` объединяет образцы в одной ветке. Именно это заменяет провал между
  `case` в C.
- `c @ образец` проверяет образец и привязывает значение к имени `c`.
- `_` подходит к чему угодно и ничего не привязывает.

Порядок веток важен: `\t`, `\r` и `\n` лежат и в диапазоне управляющих
символов, но достаются ветке `Space`, потому что она выше. Ветку, до
которой не может дойти ни одно значение, компилятор отмечает
предупреждением `unreachable_patterns`.

Константа в образце сравнивается по значению:

```rust
{{#include ../../code/ch02/src/patterns.rs:consts}}
```

<div class="box trap">

**Ловушка.** Если имени константы нет в области видимости, тот же образец
означает новую переменную, а переменная подходит к любому значению.
Забытый `use proto::*` превращает первую ветку в «всё остальное»:

```text
warning: unreachable pattern
 --> up.rs:9:9
  |
8 |         ACK => "ack", // забыли `use proto::*`: это переменная
  |         --- matches any value
9 |         NAK => "nak",
  |         ^^^ no value can reach this
  |
  = note: `#[warn(unreachable_patterns)]` (part of `#[warn(unused)]`) on by default
help: you might have meant to pattern match against the value of constant `ACK` instead of introducing a new catch-all binding
  |
8 |         proto::ACK => "ack", // забыли `use proto::*`: это переменная
  |         +++++++
```

rustc 1.97 выдаёт на этот фрагмент восемь предупреждений, включая
`non_snake_case` для «переменной» `ACK`, но всё-таки собирает программу.
Пропустить их в длинном журнале сборки легко, поэтому в CI предупреждения
запрещают (`-D warnings`, раздел [0.5](../ch00/05-checks.md)). Пример
лежит doc-тестом с `#![deny(unreachable_patterns)]` в
`code/ch02/src/patterns.rs`.

</div>

## Срезы

Образец среза проверяет длину и элементы сразу. `..` пропускает
сколько угодно элементов, а `имя @ ..` привязывает пропущенное к
подсрезу. Разбор кадра протокола выглядит как его описание:

```rust
{{#include ../../code/ch02/src/patterns.rs:slices}}
```

Вторая ветка подходит к срезу длиной от четырёх байт, который начинается
с `START`: команда, длина и сумма — по одному байту, данные — всё, что
между. Короткий кадр с правильным началом достаётся третьей ветке,
остальные непустые — четвёртой. Компилятор проверяет полноту и здесь,
перебирая длины и значения первого байта. Без последней ветки сборка
остановится:

```text
error[E0004]: non-exhaustive patterns: `&[0_u8..=125_u8]`, `&[127_u8..=u8::MAX]`, `&[0_u8..=125_u8, _]` and 5 more not covered
```

Первого байта 126 (`0x7E`) в образцах из сообщения нет: его покрывают
ветки с `START`.

Сопоставлять можно и сам срез, а не ссылку на него. Тогда элементы `Copy`
привязываются копиями:

```rust
{{#include ../../code/ch02/src/patterns.rs:first_last}}
```

<div class="box c">

**Как в C.** Тот же разбор на C — это `if (len < 4)`, `buf[1]`, `buf[2]`,
`buf + 3`, `buf[len - 1]` и проверка `buf[2] == len - 4`, где каждое
число легко сдвинуть на единицу. В образце индексов нет вовсе, а
проверку длины компилятор выводит из самого образца.

</div>

## Кортежи и таблица состояний

`match` по кортежу сопоставляет несколько значений сразу. Автомат
состояний записывается таблицей переходов, по строке на переход:

```rust
{{#include ../../code/ch02/src/patterns.rs:state_table}}
```

`if tries < 3` после образца — *охранное условие* (match guard): ветка выбирается,
только если подошёл образец и условие истинно. При проверке полноты
компилятор считает, что охранное условие может оказаться ложным, поэтому
следом нужна ветка без условия.

<div class="box trap">

**Ловушка.** Последняя ветка `(s, _) => s` делает `match` полным при любых
состояниях и событиях, но заодно отключает проверку полноты. Удалите
строку `(Link::Connecting { .. }, Event::Timeout) => Link::Down`, и
компилятор промолчит: третий таймаут достанется ветке `(s, _)`, и
устройство останется в `Connecting { tries: 3 }` навсегда. Добавьте
событие `Event::Reset`, и компилятор тоже промолчит. Ветка «всё
остальное» в таблице состояний — то же, что `default` в `switch` из
введения. Если она не нужна по смыслу, выпишите пары явно, и тогда
компилятор напомнит о каждом новом состоянии и событии.

</div>

## Режимы связывания

Когда образец без `&` сопоставляется со ссылкой, Rust неявно
разыменовывает её и привязывает части по ссылке: *режим связывания*
(binding mode) — по значению, `ref` или `ref mut` — переключается сам. Так `match counter` при
`counter: &mut Option<u32>` даёт `n: &mut u32`:

```rust
{{#include ../../code/ch02/src/patterns.rs:binding_modes}}
```

До Rust 1.26 (2018 год) образцы записывали явно: `match *counter` и
`Some(ref mut n)`, как во второй функции. [Edition][g-edition] 2024 запрещает
смешивать два способа в одном образце: явные `ref`, `mut` и `&` внутри
неявно-ссылочного образца теперь ошибка. В 2021 одно слово `mut` в таком
образце молча превращало ссылку в копию, и прочитать это из кода было
трудно:

```text
error: cannot explicitly borrow within an implicitly-borrowing pattern
 --> r.rs:3:14
  |
3 |         Some(ref mut n) => *n += 1,
  |              ^^^^^^^ explicit `ref` binding modifier not allowed when implicitly borrowing
  |
  = note: for more information, see <https://doc.rust-lang.org/reference/patterns.html#binding-modes>
note: matching on a reference type with a non-reference pattern implicitly borrows the contents
 --> r.rs:3:9
  |
3 |         Some(ref mut n) => *n += 1,
  |         ^^^^^^^^^^^^^^^ this non-reference pattern matches on a reference type `&mut _`
```

Это `match counter { Some(ref mut n) => ... }` без звёздочки. Подробнее о
том, что изменилось в 2024, — приложение
[B.4](../appendix-b.md#b4-образцы).

<div class="box check">

**Проверь себя.** Почему в `first_last` написано `match *xs`, а не
`match xs`? Какие типы получат `first` и `last` во втором случае и что
придётся поменять в ветках?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-edition]: ../appendix-f.md#edition "Редакция (edition): Версия правил языка, которую выбирает каждый крейт: 2015, 2018, 2021, 2024."
