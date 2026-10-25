# 12.4. Замыкание как результат и как поле

## `impl Fn` в результате

Тип замыкания нельзя написать, поэтому функция, которая возвращает
замыкание, называет не тип, а свойство: «что-то, что реализует `FnMut()
-> u32`». Это `impl Trait` в позиции результата из главы 10: тип
настоящий и один, компилятор его знает, а вызывающий видит только трейт.

```rust
{{#include ../../code/ch12/src/ret.rs:counter}}
```

Счётчик живёт в структуре замыкания, а структура — в переменной
вызывающего: `make_counter(10)` занимает 4 байта, кучи нет, и три вызова
дают `[11, 12, 13]`. Это функция с состоянием, которое в C пришлось бы
держать в `static` (одно на всю программу) или в структуре, которую
передают вручную.

`move` здесь обязателен. Без него замыкание держало бы ссылку на `n`, а
`n` умирает вместе с кадром `make_counter` — та самая ошибка
`start_session` из введения, только найденная при компиляции:

```console
error[E0373]: closure may outlive the current function, but it borrows `n`, which is owned by the current function
3 |     || {
  |     ^^ may outlive borrowed value `n`
4 |         n += 1;
  |         - `n` is borrowed here
note: closure is returned here
help: to force the closure to take ownership of `n` (and any other referenced variables), use the `move` keyword
3 |     move || {
  |     ++++
```

Это вывод rustc 1.97. Nightly 1.101 сообщает о том же иначе — `E0597:
n does not live long enough`, — поэтому doc-тест в
`code/ch12/src/ret.rs` проверяет, что код не компилируется, но код
ошибки не сверяет.

## Два замыкания — два типа

`impl Fn` в результате — один скрытый тип на функцию. Вернуть из двух
ветвей два разных замыкания нельзя, даже если тела одинаковы:

```console
error[E0308]: `if` and `else` have incompatible types
2 |     if invert { move |x: u8| !x ^ k } else { move |x: u8| x ^ k }
  |                 -------------------          ^^^^^^^^^^^^^^^^^^ expected closure, found a different closure
  |                 |
  |                 the expected closure
  |                 expected because of this
  = note: expected closure `{closure@r2.rs:2:17: 2:29}`
             found closure `{closure@r2.rs:2:46: 2:58}`
  = note: no two closures, even if identical, have the same type
  = help: consider boxing your closure and/or using it as a trait object
```

Выходов три, и у каждого своя цена:

```rust
{{#include ../../code/ch12/src/ret.rs:either}}
```

| Способ | Размер результата на x86-64 | Куча | Вызов |
|---|---|---|---|
| `pick_fn` — указатель на функцию | 8 | нет | косвенный |
| `pick_branch` — одно замыкание с ветвлением | 2 | нет | прямой, встраивается |
| `pick_boxed` — `Box<dyn Fn>` | 16 | да | косвенный через таблицу |

Указатель на функцию годится, только пока ничего не захвачено.
Ветвление внутри одного замыкания — самый дешёвый путь, если вариантов
немного и они известны заранее. `Box<dyn Fn>` нужен, когда вариантов
открытое множество или замыкания приходят из разных мест программы —
ровно случай реестра обработчиков. Так и предлагает компилятор в
`help:` к ошибке выше.

## Композиция

Функция может принимать замыкания и возвращать новое, собранное из них.
Тип результата — вложенные структуры, и снаружи его не видно:

```rust
{{#include ../../code/ch12/src/ret.rs:compose}}
```

`pipeline(100, 3)` превращает `[90, 200, 1000]` в `[-30, 300, 1000]` и
занимает 8 байт: `offset` и `gain`. Ограничитель `clamp` ничего не
захватывает и места не занимает, а структура `compose` — только сумма
своих полей. Вызов `p(x)` после встраивания — три инструкции
арифметики, без единого вызова; так это и выглядит в разделе
[12.5](05-codegen.md).

## Какие времена жизни захватывает `impl Trait`

В редакции 2024 непрозрачный тип в результате считается зависящим от
всех параметров функции, в том числе от времён жизни ссылок, даже если
скрытый тип ими не пользуется. Функция ниже берёт `&i32`, копирует
значение и возвращает замыкание без ссылок, но без `use<>` вызывающий
не может менять `k`, пока результат жив:

```console
error[E0506]: cannot assign to `k` because it is borrowed
3 | let f = scaler(&k);
  |                -- `k` is borrowed here
4 | k += 1;
  | ^^^^^^ `k` is assigned to here but it was already borrowed
5 | assert_eq!(f(2), 6);
  |            - borrow later used here
note: this call may capture more lifetimes than intended, because Rust 2024 has adjusted the `impl Trait` lifetime capture rules
help: use the precise capturing `use<...>` syntax to make the captures explicit
7 | fn scaler(k: &i32) -> impl Fn(i32) -> i32 + use<> {
  |                                           +++++++
```

`use<…>` перечисляет, от чего результат зависит на самом деле; `use<>`
— ни от чего:

```rust
{{#include ../../code/ch12/src/ret.rs:use-bound}}
```

Обмануть компилятор `use<>` не даёт. Если замыкание всё же держит
ссылку, ошибка будет в самой функции:

```console
error[E0700]: hidden type for `impl Fn(i32) -> i32` captures lifetime that does not appear in bounds
1 | fn scaler(k: &i32) -> impl Fn(i32) -> i32 + use<> {
  |              ----     --------------------------- opaque type defined here
  |              |
  |              hidden type `{closure@r4.rs:2:5: 2:13}` captures the anonymous lifetime defined here
2 |     move |x| x * *k
  |     ^^^^^^^^^^^^^^^
help: add `'_` to the `use<...>` bound to explicitly capture it
```

До редакции 2024 правило было обратным: `impl Trait` не захватывал
времена жизни, если их не назвать, и приходилось писать `+ '_`.
`cargo fix --edition` при переходе добавляет `use<…>` туда, где новое
правило меняло бы смысл. Оба примера — doc-тесты с кодами `E0506` и
`E0700`.

## Замыкание в поле структуры

Хранить замыкание в структуре можно двумя способами, как и любой
полиморфный объект из глав 10 и 11: параметром типа или `Box<dyn …>`.
Параметр типа не стоит ничего сверх захваченного:

```rust
{{#include ../../code/ch12/src/ret.rs:field}}
```

Подавитель дребезга получает обработчик, который пишет в две
переменные вызывающего, и занимает три слова: две ссылки из замыкания,
`bool` и `u8` с выравниванием. Это раскладка rustc 1.97: порядок полей
`repr(Rust)` не обещан, и под `-Zrandomize-layout` при некоторых
зёрнах (на nightly 1.101.0 от 2026-10-23 — при зерне 2; почему это
зависит от версии — раздел [6.2](../ch06/02-repr-c.md)) тот же тип
занимает 32 байта, потому что два байта оказываются по разные стороны
от ссылок. Поэтому размер проверяет отдельный тест
`observed_debounce_size`, который в задании CI со случайной раскладкой
пропускается. Тест подаёт `[true, false, true, true,
true, true, false, false, false]` и получает два изменения уровня,
последнее — в `false`. Время жизни заёмов обработчика привязано к
`Debounce`, и пока подавитель жив, читать `changes` снаружи нельзя —
поэтому тест держит его во внутреннем блоке.

Поле-замыкание вызывают в скобках: `(self.on_change)(level)`. Запись
`self.on_change(level)` — поиск метода с таким именем, и компилятор
подсказывает:

```console
error[E0599]: no method named `on_click` found for mutable reference `&mut Button<F>` in the current scope
3 |     fn press(&mut self) { self.on_click() }
  |                                ^^^^^^^^ field, not a method
help: to call the type parameter stored in `on_click`, surround the field access with parentheses
3 |     fn press(&mut self) { (self.on_click)() }
  |                           +             +
```

<div class="box trap">

**Ловушка.** Привычка ставить границы только туда, где они нужны,
подводит с замыканиями. Если конструктор лежит в `impl<F>` без границы,
а `F: FnMut(bool)` требует только метод, который зовёт поле, тип
параметра замыкания вывести не из чего: тело замыкания компилятор
проверяет в месте создания, и граница на другом методе туда не
доходит.

```console
error[E0282]: type annotations needed
10 | let mut b = Button::new(|long| longs += long.then_some(1).unwrap_or(0));
   |                          ^^^^           ---- type must be known at this point
help: consider giving this closure parameter an explicit type
```

Граница на `impl` с `new`, как у `Debounce`, или на самом `new`
решает это, и аннотация `|long: bool|` не нужна. Пример — doc-тест с
кодом `E0282` в `code/ch12/src/ret.rs`.

</div>

<div class="box check">

**Проверь себя.** Сколько байт займёт `compose(compose(shift, scale),
clamp)`, если `clamp` захватит пределы `lo: i32` и `hi: i32`? Почему
`make_counter` нельзя вызвать через неизменяемую привязку `let c =
make_counter(0);`? Что изменится в типе `Debounce`, если обработчик
положить в `Box<dyn FnMut(bool)>`, и что — в его размере на Cortex-M3?

</div>
