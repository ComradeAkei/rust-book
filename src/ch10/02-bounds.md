# 10.2. Ограничения и `where`

## Ограничения на связанные типы

Среднее отсчётов АЦП должно работать для массива `i16`, для среза `u8` и
для [итератора][g-iterator], который читает регистр. Общего у них одно: это источник
значений, каждое из которых без потерь расширяется до `i32`.

```rust
{{#include ../../code/ch10/src/bounds.rs:where}}
```

`I: IntoIterator` говорит, что `samples` можно обойти циклом `for`.
`I::Item: Into<i32>` ограничивает уже не сам `I`, а его [ассоциированный
тип][g-assoc-type] из раздела [9.2](../ch09/02-associated.md): элементы. Такое
ограничение не записать в угловых скобках рядом с `I`, поэтому оно стоит
в `where` после сигнатуры. С Rust 1.79 то же можно записать короче,
`I: IntoIterator<Item: Into<i32>>`, но `where` остаётся обычным выбором,
когда ограничений больше двух: сигнатура не расползается.

Какие типы подходят, решает `Into<i32>`: его реализуют `i8`, `u8`, `i16`,
`u16`, `i32` и даже `bool`, но не `u32` и не `i64`, где расширение не
всегда возможно. Тест вызывает `average` с массивом `[i16; 4]`, итератором
`copied()` по срезу `u8`, вектором `u16` и пустым итератором `i8`.
Попытка передать `[1u32, 2]` — ошибка E0277 в месте вызова: «the trait
`From<u32>` is not implemented for `i32`». Сумма накапливается в `i64`:
`n` значений `i32` в сумме могут не поместиться в `i32`.

## Условные методы

Ограничение можно поставить не на функцию, а на блок `impl`. Тогда
методы блока есть только у тех членов семейства, у которых параметр
ограничениям отвечает:

```rust
{{#include ../../code/ch10/src/bounds.rs:labeled}}
```

Определение `Labeled<T>` ничего от `T` не требует: хранить можно что
угодно. `Display` реализован для `Labeled<T>`, только если он есть у `T`,
и вместе с ним приходит `to_string`. `exceeds` есть, если `T` можно
сравнивать. Для `Labeled<f32>` доступно всё, для `Labeled<[u8; 2]>` —
только `exceeds`: массивы сравниваются, но не печатаются через `{}`.
Для типа без обоих свойств остаются только поля:

```text
error[E0599]: `Labeled<Raw>` doesn't implement `std::fmt::Display`
 --> e0599.rs:6:46
  |
6 |     Labeled { name: "АЦП", value: Raw(812) }.to_string()
  |                                              ^^^^^^^^^ method cannot be called on `Labeled<Raw>` due to unsatisfied trait bounds
  |
  = note: the following trait bounds were not satisfied:
          `Labeled<Raw>: std::fmt::Display`
          which is required by `Labeled<Raw>: ToString`
```

Компилятор прослеживает всю цепочку: `to_string` есть у любого `T:
Display` (это [общая реализация][g-blanket-impl] из раздела [9.4](../ch09/04-coherence.md)),
`Labeled<Raw>` был бы `Display`, если бы `Raw` был `Display`, а он нет.

## Где ограничениям не место

Ограничение можно написать и в определении типа, но тогда его придётся
повторять в каждом `impl`, даже там, где оно не нужно:

```text
error[E0277]: the trait bound `T: Ord` is not satisfied
 --> e0277b.rs:3:9
  |
3 | impl<T> Sorted<T> {
  |         ^^^^^^^^^ the trait `Ord` is not implemented for `T`
  |
note: required by a bound in `Sorted`
 --> e0277b.rs:1:22
  |
1 | pub struct Sorted<T: Ord>(Vec<T>);
  |                      ^^^ required by this bound in `Sorted`
help: consider restricting type parameter `T` with trait `Ord`
  |
3 | impl<T: std::cmp::Ord> Sorted<T> {
  |       +++++++++++++++
```

Метод `len` порядок элементов не использует, но без `T: Ord` блок не
собирается: ограничение типа переходит на всё, что его упоминает, вплоть
до каждой функции, принимающей `Sorted<T>`. Поэтому в стандартной
библиотеке у `Vec<T>`, `HashMap<K, V>` и `BTreeMap<K, V>` в определениях
ограничений нет, а `K: Hash + Eq` и `K: Ord` стоят на блоках `impl` с
методами, которым они нужны. Ограничение в определении оправдано, только
когда без него нельзя записать сами поля, например поле типа `I::Item`
требует `I: Iterator`.

<div class="box trick">

**Трюк.** Каждый параметр типа неявно ограничен `T: Sized`: значение
`T` должно иметь размер, известный при компиляции, иначе его не положить
на стек. Для функций, которые принимают `T` только по ссылке, это
ограничение лишнее, и его снимают записью `T: ?Sized`:
`fn len_of<T: ?Sized + AsRef<[u8]>>(x: &T)` принимает и `&str`, и `&[u8]`.
[`Sized`][g-sized] разбирал раздел [9.5](../ch09/05-markers.md).

</div>

## `T: 'static`

Ограничение бывает и [временем жизни][g-lifetime]. `T: 'a` значит, что все ссылки
внутри `T` живут не меньше `'a`. Чаще всего встречается `T: 'static`:

```rust
{{#include ../../code/ch10/src/bounds.rs:static}}
```

`'static` здесь — свойство типа, а не значения. `String` ссылок не
содержит и ему отвечает, даже если сама строка локальная и умрёт в конце
функции: `assert_static(&s)` собирается. А `&String` — ссылка со
временем жизни `s`, и её тип ограничению не отвечает:

```text
error[E0597]: `s` does not live long enough
 --> e0597.rs:5:13
  |
4 |     let s = String::from("локальная строка");
  |         - binding `s` declared here
5 |     let r = &s;
  |             ^^ borrowed value does not live long enough
6 |     assert_static(&r);
  |     ----------------- argument requires that `s` is borrowed for `'static`
7 | }
  | - `s` dropped here while still borrowed
  |
note: requirement that the value outlives `'static` introduced here
 --> code/ch10/src/bounds.rs:85:24
```

`T: 'static` требуют `std::thread::spawn` (поток может пережить
вызвавшую функцию, глава 20) и `Box<dyn Any>` (глава 11): им нужен тип,
значения которого можно держать сколько угодно, не следя за чужими
заёмами.

<div class="box trap">

**Ловушка.** `T: 'static` не значит «значение живёт до конца программы».
`String`, `Vec<u8>` и `u32` — `'static`-типы, и их значения создаются и
уничтожаются как обычно. Не значит оно и «ссылок нет вовсе»: `&'static
str` ограничению отвечает. Запрещены только ссылки, которые могут
повиснуть. Путаница рождает лишние `'static` в сигнатурах, которые потом
не дают передать в функцию [заимствованные][g-borrowing] данные.

</div>

<div class="box check">

**Проверь себя.** Почему `average` нельзя вызвать с `&[i16]` без
`copied()`, и какая ошибка будет? Какие методы есть у `Labeled<Labeled<u8>>`?
Соберётся ли `assert_static(&"литерал")`, и чем этот случай отличается от
`assert_static(&r)` выше?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-assoc-type]: ../appendix-f.md#assoc-type "Ассоциированный тип (associated type): Тип, который называет каждая реализация трейта: type Sample в трейте Source."
[g-blanket-impl]: ../appendix-f.md#blanket-impl "Общая реализация (blanket impl): Реализация трейта сразу для всех типов, удовлетворяющих ограничению: impl ToString for T."
[g-borrowing]: ../appendix-f.md#borrowing "Заимствование (borrowing): Создание ссылки на значение без передачи владения: &x — разделяемое, &mut x — уникальное."
[g-iterator]: ../appendix-f.md#iterator "Итератор (iterator): Тип, реализующий трейт Iterator: состояние обхода с методом next, который выдаёт следующий элемент или None."
[g-lifetime]: ../appendix-f.md#lifetime "Время жизни (lifetime): Имя для участка программы, на котором ссылка действительна: 'a в &'a str."
[g-sized]: ../appendix-f.md#sized "Sized, ?Sized: Sized — маркерный трейт типов, размер которых известен при компиляции; компилятор неявно требует его от каждого параметра типа."
