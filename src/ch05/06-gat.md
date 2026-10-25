# 5.6. GAT и одалживающие итераторы

## Итератор, который отдаёт свой буфер

Строки конфигурации надо нормализовать — убрать комментарий и все
пробелы — и отдавать по одной. Выделять `String` на каждую строку
незачем: достаточно одного буфера, который переписывается на каждом
шаге. Итератор с таким буфером на `Iterator` не пишется:

```rust,ignore
impl<'s> Iterator for Normalize<'s> {
    type Item = &'s str;

    fn next(&mut self) -> Option<&'s str> {
        let line = self.lines.next()?;
        self.buf.clear();
        self.buf.extend(line.chars().filter(|c| !c.is_whitespace()));
        Some(&self.buf)
    }
}
```

```text
error: lifetime may not live long enough
  --> gat_0.rs:15:9
   |
 8 | impl<'s> Iterator for Normalize<'s> {
   |      -- lifetime `'s` defined here
...
11 |     fn next(&mut self) -> Option<&'s str> {
   |             - let's call the lifetime of this reference `'1`
...
15 |         Some(&self.buf)
   |         ^^^^^^^^^^^^^^^ method was supposed to return data with lifetime `'s` but it is returning data with lifetime `'1`
```

`Item` — один тип на весь итератор, и в нём нельзя сослаться на
заимствование `&mut self` в конкретном вызове `next`. Можно назвать
только `'s`, время жизни текста, но буфер принадлежит итератору, а не
тексту. И отказ справедлив: `Iterator` разрешает `collect()`, то есть
держать все элементы одновременно, а здесь все они смотрели бы в один
буфер, который каждый `next` переписывает. Это опять `config.c` из
введения.

## Обобщённые ассоциированные типы

С Rust 1.65 ассоциированный тип может иметь свои параметры — это *GAT* (generic associated types). Тип элемента становится семейством типов,
по одному на каждое время жизни заимствования:

```rust
{{#include ../../code/ch05/src/gat.rs:lender}}
```

- `Item<'a>` — тип элемента, взятого через заимствование итератора на
  `'a`. В `next` он `Item<'_>`: время жизни берётся у `&mut self` по
  правилу элизии 3. Элемент держит итератор, пока жив.
- `where Self: 'a` — элемент не может жить дольше итератора, из которого
  взят. Раз `next` получает `Item<'_>` через `&mut self`, компилятор
  требует это условие в объявлении трейта: без него — ошибка «missing
  required bound on `Item`» с подсказкой `where Self: 'a`.
- Такой итератор называют *одалживающим* (lending iterator): он не
  отдаёт элементы, а одалживает их до следующего `next`.

Пользоваться им можно только циклом `while let`. `for` работает с
`IntoIterator`, а значит, с `Iterator`:

```rust
{{#include ../../code/ch05/src/gat.rs:use}}
```

Тест `lender` проверяет, что вторая и третья строки лежат по одному
адресу — это тот самый буфер. Удержать строку после следующего `next`
компилятор не даст (E0499, как у `word` в разделе [5.1](01-elision.md),
и так же у окон в задаче [5.D](08-tasks.md)).

## Ловушка: `for<'a>` на GAT значит `'static`

Функция, которая ищет ключ, должна заглянуть внутрь элемента. Для
обобщённого `L` естественно потребовать, чтобы элемент читался как
строка при любом заимствовании:

```rust
{{#include ../../code/ch05/src/gat.rs:trap}}
```

Функция компилируется. Тест с текстом-литералом проходит. А со строкой
из кучи — нет (файл `code/ch05/rejected/gat_static.rs`, его собирает
тест `tests/rejected.rs`):

```text
error[E0521]: borrowed data escapes outside of function
  --> rejected/gat_static.rs:55:5
   |
54 | pub fn bauds(text: &str) -> usize {
   |              ----  - let's call the lifetime of this reference `'1`
   |              |
   |              `text` is a reference that is only valid in the function body
55 |     count_keys_hrtb(Normalize::new(text), "baud")
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |     |
   |     `text` escapes the function body here
   |     argument requires that `'1` must outlive `'static`
   |
   = note: requirement occurs because of the type `Normalize<'_>`, which makes the generic argument `'_` invariant
   = note: the struct `Normalize<'s>` is invariant over the parameter `'s`
   = help: see <https://doc.rust-lang.org/nomicon/subtyping.html> for more information about variance
note: due to a current limitation of the type system, this implies a `'static` lifetime
  --> rejected/gat_static.rs:43:5
   |
43 |     for<'a> L::Item<'a>: AsRef<str>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Последняя заметка называет причину. `for<'a>` пробегает *все* времена
жизни, в том числе `'static`, а `Item<'static>` по условию `where Self:
'a` требует `L: 'static`. Сказать «для всех `'a`, которые не длиннее
`L`» в `for<..>` нельзя, и компилятор делает худший вывод. Заметки про
инвариантность — следствие ловушки из раздела [5.4](04-variance.md):
`Normalize` хранит `str::Lines`, а он инвариантен. Пояснение «due to a
current limitation» rustc 1.97 печатает, только если граница записана в
том же [крейте][g-crate]; при вызове из другого крейта (doc-тест в
`code/ch05/src/gat.rs`) остаётся одна E0521.

<div class="box trick">

**Трюк.** Условие «`Self` живёт не меньше `'a`» можно спрятать в
параметр трейта. Тип `&'a Self` существует, только если `Self: 'a`, и
компилятор выводит это условие везде, где тип упомянут:

```rust
{{#include ../../code/ch05/src/gat.rs:workaround}}
```

Параметр по умолчанию `Bound = &'a Self` никто не задаёт явно, он нужен
только ради неявного условия. Теперь `for<'a> LendItem<'a, Item = &'a
str>` пробегает лишь допустимые `'a`, и `count_keys` работает со строкой
из кучи — это проверяет тест `count`. Цена — второй трейт и менее
очевидные сигнатуры, поэтому `Lender` с простым GAT остаётся основным
вариантом, а обход нужен, когда обобщённому коду требуется граница на
элемент.

</div>

<div class="box check">

**Проверь себя.** Почему `Records` из раздела [5.2](02-structs.md) —
обычный `Iterator`, а `Normalize` — нет, хотя оба отдают срезы? Что
изменится, если `Normalize` будет отдавать `String`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-crate]: ../appendix-f.md#crate "Крейт (crate): Единица компиляции Rust: одно дерево модулей, один вызов rustc, один результат — библиотека или программа."
