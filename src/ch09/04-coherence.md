# 9.4. Когерентность и правило сироты

## Одна реализация на пару

Вызов `x.fmt(f)` для `x: Vec<i32>` должен означать одно и то же во всей
программе. Если бы библиотека журнала реализовала `Display` для
`Vec<i32>` одним способом, а библиотека телеметрии — другим, то в
прошивке с обеими библиотеками компилятору пришлось бы выбирать, и
результат зависел бы от того, какой [крейт][g-crate] он увидел первым. Rust
требует *когерентности* (coherence): для каждой пары «трейт — тип» во всей
программе не больше одной реализации.

Проверить это целиком можно только при сборке всей программы, а
крейты компилируются по отдельности. Поэтому правило сформулировано
локально: крейту разрешено писать только такие реализации, которые не
может написать никто другой. Это *правило сироты* (orphan rule): чужой
трейт для чужого типа — «сирота», у которого нет законного владельца.

## Правило сироты

Упрощённо: `impl Трейт for Тип` разрешён, если трейт или тип объявлен в
текущем крейте. Своя `Readings` может получить `Display`, свой `Sensor`
можно реализовать для `u32`, а `Display` для `Vec<i32>` — нельзя никому,
кроме `alloc` и `core`. [Doc-тест][g-doctest] — отдельный крейт, для него `ch09` чужой:

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
 --> e0117.rs:1:1
  |
1 | impl core::fmt::Display for ch09::coherence::Readings {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^-------------------------
  |                             |
  |                             `Readings` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
  = note: for more information see https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
  = note: define and implement a trait or new type instead
```

Точное правило (RFC 2451) учитывает параметры трейта и обобщённые
реализации. Запишем реализацию как `impl<P1, …> Трейт<T1, …, Tn> for T0`.
Она разрешена, если трейт свой, или если среди `T0, T1, …, Tn` есть
*локальный* тип и до первого такого типа не встречается *непокрытый*
параметр `P`. Определения такие:

- Локальный тип — объявленный в этом крейте. `Vec<Local>` локальным не
  считается: `Vec` чужой, а то, что внутри, не важно. Исключение —
  *фундаментальные* обёртки `&T`, `&mut T`, `Box<T>` и [`Pin<T>`][g-pinning]: они
  локальны, если локален `T`.
- Непокрытый параметр — параметр реализации, который стоит сам по себе, а
  не внутри какого-то типа: в `impl<T> Трейт for T` параметр `T`
  непокрыт, в `impl<T> Трейт for Vec<T>` — покрыт.

Второе условие закрывает обобщённые реализации чужого трейта:

```text
error[E0210]: type parameter `T` must be used as the type parameter for some local type (e.g., `MyStruct<T>`)
 --> e0210.rs:2:6
  |
2 | impl<T: Sensor> core::fmt::Display for T {
  |      ^ type parameter `T` must be used as the type parameter for some local type
  |
  = note: implementing a foreign trait is only possible if at least one of the types for which it is implemented is local
  = note: only traits defined in the current crate can be implemented for a type parameter
```

Здесь `T0 = T` — непокрытый параметр, а локальных типов нет вовсе. Такая
реализация подошла бы и к типам из других крейтов, и два крейта могли бы
написать её одновременно.

## Обёртка newtype

Если нужен чужой трейт для чужого типа, тип оборачивают в свой:

```rust
{{#include ../../code/ch09/src/coherence.rs:newtype}}
```

`Readings` — локальный тип, и `Display` для него законен:
`Readings(vec![2150, -405, -5])` печатается как `21.50, -4.05, -0.05`.
Обёртка из одного поля ничего не стоит во время работы, у неё та же
раскладка, что у `Vec<i32>`. Цена — в удобстве: методов `Vec` у
`Readings` нет, к ним обращаются через `.0`. Соблазн вернуть их через
`Deref` разобран в разделе [9.7](07-tricks.md).

## Локальный тип в параметре трейта

Точное правило разрешает и то, что упрощённое запрещает:

```rust
{{#include ../../code/ch09/src/coherence.rs:local-param}}
```

`From` и `i32` чужие, но трейт здесь не `From`, а `From<Celsius>`, и
`T1 = Celsius` — локальный тип без непокрытых параметров перед ним.
Никакой другой крейт `From<Celsius>` написать не может, потому что не
может назвать `Celsius` своим. Реализация `Into` при этом приходит
бесплатно, из `core`:

```rust
impl<T, U: From<T>> Into<U> for T { … }
```

Поэтому `let raw: i32 = Celsius(2150).into();` компилируется, хотя
`Into` никто не писал. Отсюда совет стандартной библиотеки: реализуйте
`From`, а `Into` получится сам.

## Общие реализации

Реализация для всех типов, удовлетворяющих ограничению, называется
*общей реализацией* (blanket impl). Свой трейт так реализовать можно:

```rust
{{#include ../../code/ch09/src/coherence.rs:blanket}}
```

Теперь любой `Sensor`, в том числе ещё не написанный, умеет
`describe`. Стандартная библиотека полна таких реализаций: `ToString`
для всего, что реализует `Display`, `Into` из `From`, `From<T> for T`.

Обратная сторона: общая реализация занимает место частных. Своя
реализация `Describe` для `Thermo` рядом с общей — ошибка, даже в том
же крейте:

```text
error[E0119]: conflicting implementations of trait `Describe` for type `Thermo`
 --> e0119a.rs:4:1
  |
3 | impl<T: Sensor> Describe for T { fn describe(&self) -> &'static str { "датчик" } }
  | ------------------------------ first implementation here
4 | impl Describe for Thermo { fn describe(&self) -> &'static str { "термодатчик" } }
  | ^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Thermo`
```

`Thermo` реализует `Sensor`, поэтому для него подходят обе реализации.
Механизм, который выбрал бы более частную (*специализация*,
specialization), в стабильном Rust не доступен.

## Будущие реализации

Компилятор считает пересечением и то, чего пока нет:

```text
error[E0119]: conflicting implementations of trait `Describe` for type `Option<u8>`
 --> e0119b.rs:3:1
  |
2 | impl<T: core::fmt::Display> Describe for T { fn describe(&self) -> &'static str { "текст" } }
  | ------------------------------------------ first implementation here
3 | impl Describe for Option<u8> { fn describe(&self) -> &'static str { "может быть" } }
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Option<u8>`
  |
  = note: upstream crates may add a new impl of trait `std::fmt::Display` for type `std::option::Option<u8>` in future versions
```

Сегодня `Option<u8>` не реализует `Display`, и реализации не
пересекаются. Но `core` может добавить `Display` для `Option<u8>` в
следующей версии, и тогда чужой код сломается без единой правки. Поэтому
о чужих типах компилятор рассуждает осторожно: «не реализует сейчас» не
значит «не реализует никогда». О своих типах он знает точно: если у
локального `Raw` нет `Display`, добавить его может только этот крейт, и
пара `impl<T: Display> Describe for T` и `impl Describe for Raw`
компилируется (doc-тест в `coherence.rs`).

<div class="box hood">

**Под капотом.** Осторожность следует из правил semver для Rust (RFC
1105): добавить реализацию трейта для своего типа — совместимое
изменение, младшая версия. Значит, отсутствие реализации у чужого типа
ничего не обещает. Исключение — фундаментальные типы и трейты, помеченные
внутренним атрибутом `#[fundamental]`: `Box`, `&`, `&mut`, `Pin`, трейты
[`Fn`][g-fn-traits], `FnMut`, `FnOnce` и [`Sized`][g-sized]. Для них добавить общую реализацию —
несовместимое изменение, и компилятор вправе рассуждать об их
отсутствии.

</div>

<div class="box trap">

**Ловушка.** Добавить общую реализацию в библиотеку — несовместимое
изменение, даже если ни одна сигнатура не тронута. Пусть версия 1.0
крейта датчиков объявляет `Describe`, а пользователь написал `impl
Describe for MyThermo`, где `MyThermo: Sensor`. Версия 1.1 добавляет
`impl<T: Sensor> Describe for T`, и код пользователя перестаёт
компилироваться с E0119. Общая реализация — решение, которое
принимается при проектировании трейта, а не в патче.

</div>

<div class="box c">

**Как в C.** В C нет реализаций, и проблемы нет. В C++ она есть: две
единицы трансляции с разными явными специализациями одного шаблона или
разными определениями одной inline-функции нарушают правило одного
определения (ODR). Это неопределённое поведение, и компоновщик
обязан о нём сообщать не больше, чем о `-fcommon` из главы 8, то есть
никак. Правило сироты делает такую ситуацию невозможной для трейтов:
у реализации всегда один законный автор.

</div>

<div class="box check">

**Проверь себя.** Можно ли в крейте, где объявлен `Celsius`, написать
`impl From<Vec<Celsius>> for i32`? А `impl<T> From<Celsius> for Vec<T>`?
Почему в крейте, где объявлен `Readings`, `impl Display for
Box<Readings>` разрешён, а `impl Display for Vec<Readings>` — нет?
Задача [9.A](08-tasks.md) — шесть таких вопросов с проверкой
компилятором.

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-crate]: ../appendix-f.md#crate "Крейт (crate): Единица компиляции Rust: одно дерево модулей, один вызов rustc, один результат — библиотека или программа."
[g-doctest]: ../appendix-f.md#doctest "doc-тест (doctest): Пример кода в документирующем комментарии ///, который cargo test собирает и запускает как тест."
[g-fn-traits]: ../appendix-f.md#fn-traits "Fn, FnMut, FnOnce: Трейты вызова, которые компилятор реализует для замыканий, функций и указателей на функции."
[g-pinning]: ../appendix-f.md#pinning "Закрепление (pinning): Обещание, что значение не переместится в памяти, пока не будет уничтожено."
[g-sized]: ../appendix-f.md#sized "Sized, ?Sized: Sized — маркерный трейт типов, размер которых известен при компиляции; компилятор неявно требует его от каждого параметра типа."
