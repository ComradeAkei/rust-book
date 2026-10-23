# 5.4. Вариантность и `PhantomData`

## Длинное вместо короткого

Если `'long` длится не меньше `'short` (пишется `'long: 'short`), то
ссылку `&'long T` можно отдать туда, где ждут `&'short T`: она
действительна на всём нужном участке и ещё дольше. Это отношение
подтипа, и в Rust оно бывает только между временами жизни.

```rust
{{#include ../../code/ch05/src/variance.rs:covariant}}
```

`"uart0"` имеет тип `&'static str`, результат — `&'a str`, и сужение
происходит молча. Вопрос в том, как подтип проходит сквозь составные
типы. Если `&'long str` годится вместо `&'short str`, годится ли
`Option<&'long str>` вместо `Option<&'short str>`? `Vec<&'long str>`?
`&mut &'long str`? Ответ на этот вопрос для каждого параметра типа
называется *вариантностью*:

- *ковариантен* — подтип проходит в ту же сторону: `F<'long>` годится
  вместо `F<'short>`. Таковы `&'a T`, `Option<T>`, `Vec<T>`, `Box<T>`;
- *контравариантен* — в обратную сторону: `F<'short>` годится вместо
  `F<'long>`. Так ведёт себя аргумент указателя на функцию;
- *инвариантен* — подходит только точное совпадение.

## `&mut T` инвариантен по `T`

```rust
{{#include ../../code/ch05/src/variance.rs:invariant}}
```

Пусть `name: &'static str`, и в `set` передают `&mut name` и ссылку на
локальную строку. Если бы `&mut &'static str` годился вместо
`&mut &'a str`, функция записала бы в `name` короткую ссылку, а после
выхода из блока `name` по-прежнему имела бы тип `&'static str` и
указывала бы в освобождённую память. Компилятор не пускает:

```text
error[E0597]: `local` does not live long enough
 --> variance_0.rs:8:20
  |
5 | let mut name: &'static str = "по умолчанию";
  |               ------------ type annotation requires that `local` is borrowed for `'static`
6 | {
7 |     let local = String::from("из конфигурации");
  |         ----- binding `local` declared here
8 |     set(&mut name, &local);
  |                    ^^^^^^ borrowed value does not live long enough
9 | }
  | - `local` dropped here while still borrowed
```

Обратите внимание, куда показывает компилятор: на аннотацию типа `name`.
`&'a mut T` ковариантен по `'a` (саму ссылку можно укоротить), но
инвариантен по `T`. Поэтому `'a` в `set` обязано совпасть с `'static`,
и `&local` должен жить вечно. Тест `invariance` показывает, что без
аннотации `'static` тот же вызов проходит: `name` — тоже локальная, и
общий срок находится.

Это же правило касается всего, через что можно писать по `&`: `Cell<T>`,
`RefCell<T>`, `Mutex<T>` и всего, что построено на `UnsafeCell<T>`
(глава [14](../ch14/05-cells.md)). Сузить время жизни внутри ячейки нельзя:

```text
error: lifetime may not live long enough
 --> variance_1.rs:6:5
  |
5 | fn shorten<'a>(c: &'a Cell<&'static str>) -> &'a Cell<&'a str> {
  |            -- lifetime `'a` defined here
6 |     c
  |     ^ returning this value requires that `'a` must outlive `'static`
  |
  = note: requirement occurs because of the type `Cell<&str>`, which makes the generic argument `&str` invariant
  = note: the struct `Cell<T>` is invariant over the parameter `T`
```

Ошибки без кода, как эта, rustdoc проверяет только как «не
компилируется» (`compile_fail` без кода). Текст сообщения — из rustc 1.97.

## Контравариантность

```rust
{{#include ../../code/ch05/src/variance.rs:contra}}
```

Обработчик, который принимает строку любой длины жизни, годится туда,
где ему будут давать только `&'static str`. Обратное неверно: обработчик,
написанный для `&'static str`, мог бы сохранить ссылку в глобальную
переменную, и короткая ссылка там стала бы висячей. `fn widen<'a>(f:
fn(&'static str) -> usize) -> fn(&'a str) -> usize` не компилируется —
doc-тест в `code/ch05/src/variance.rs`.

## Сводка

| Тип | по `'a` | по `T` |
|---|---|---|
| `&'a T` | ковариантен | ковариантен |
| `&'a mut T` | ковариантен | инвариантен |
| `*const T` | | ковариантен |
| `*mut T` | | инвариантен |
| `Box<T>`, `Vec<T>`, `Option<T>` | | ковариантен |
| `Cell<T>`, `RefCell<T>`, `UnsafeCell<T>` | | инвариантен |
| `fn(T) -> U` | | контравариантен по `T`, ковариантен по `U` |
| `dyn Trait<T> + 'a` | ковариантен | инвариантен |
| `PhantomData<T>` | | как `T` |

Вариантность своих типов компилятор выводит из полей: структура
ковариантна по параметру, если все поля с ним ковариантны, инвариантна,
если хотя бы одно поле инвариантно, и так далее. Написать вариантность
явно нельзя.

<div class="box trap">

**Ловушка.** Вывод по полям не заглядывает в ассоциированные типы. Поле
типа `<P as Pattern>::Searcher<'a>` делает структуру инвариантной по
`'a`, что бы ни было за проекцией. Так инвариантны `str::Lines<'a>` и
`str::Split<'a, P>` из стандартной библиотеки, а `str::Chars<'a>` —
ковариантен. Обёртка над `Lines` из раздела [5.6](06-gat.md) поэтому
тоже инвариантна, и rustc упоминает это в сообщениях об ошибках. Задача
[5.C](08-tasks.md) проверяет таблицу на десяти типах.

</div>

## `PhantomData`: время жизни без ссылки в полях

Буфер, отданный каналу DMA, описывают адрес и длина: их записывают в
регистры контроллера. Ссылки в полях такой структуры нет, а время жизни
нужно — буфер нельзя освобождать и трогать, пока идёт передача. Если
объявить параметр без поля, компилятор откажет:

```text
error[E0392]: lifetime parameter `'a` is never used
 --> variance_3.rs:3:18
  |
3 | struct DmaWindow<'a> {
  |                  ^^ unused lifetime parameter
  |
  = help: consider removing `'a`, referring to it in a field, or using a marker such as `PhantomData`
```

Причина — вариантность: её выводят из полей, а полей с `'a` нет.
`PhantomData<X>` — поле нулевого размера (раздел
[6.5](../ch06/05-zst.md)), которое для проверки типов выглядит как поле
типа `X`:

```rust
{{#include ../../code/ch05/src/variance.rs:phantom}}
```

`PhantomData<&'a mut [u8]>` даёт окну всё, что дала бы ссылка `&'a mut
[u8]`: ковариантность по `'a` и уникальное заимствование буфера. Пока
окно живо, буфер нельзя ни освободить, ни прочитать:

```text
error[E0505]: cannot move out of `rx` because it is borrowed
 --> variance_4.rs:7:6
  |
5 | let mut rx = vec![0u8; 16];
  |     ------ binding `rx` declared here
6 | let w = DmaWindow::new(&mut rx);
  |                        ------- borrow of `rx` occurs here
7 | drop(rx); // контроллер ещё пишет в буфер
  |      ^^ move out of `rx` occurs here
8 | let _ = w.finish();
  |         - borrow later used here
```

`finish` поглощает окно и возвращает `&'a mut [u8]` — передача
закончена, буфер снова у программы. Внутри один вызов
`slice::from_raw_parts_mut` в блоке `unsafe`: собрать ссылку из адреса и
длины компилятор не может проверить, и обоснование записано в
комментарии `SAFETY` (правила таких блоков — глава [16](../ch16.md)).
Тест моделирует запись контроллера через `ptr::copy_nonoverlapping` и
под Miri проходит в обеих моделях алиасинга. Почему в полях окна сырой
указатель, а не `&'a mut [u8]`, объяснит глава [16](../ch16.md):
уникальная ссылка на память, в которую пишет DMA, — уже неопределённое
поведение. Барьеры и `volatile` для настоящего контроллера добавит
глава [17](../ch17.md).

<div class="box trap">

**Ловушка.** Время жизни, которое `unsafe`-код берёт «из воздуха», ничем
не ограничено. Если объявить `fn finish<'x>(self) -> &'x mut [u8]` с
собственным параметром, вызывающий получит любое время жизни, какое
попросит, — в том числе `'static` для буфера на стеке. Такая функция
компилируется:

```rust,ignore
pub fn dangling() -> &'static mut [u8] {
    let mut rx = [0u8; 4];
    let w = Win::new(&mut rx); // окно без PhantomData и без 'a
    w.finish()                 // &'static mut на мёртвый стек
}
```

Время жизни результата должно быть привязано к владельцу памяти — здесь
к `'a` окна. Этот пункт есть в чек-листе приложения
[C](../appendix-c.md).

</div>

<div class="box check">

**Проверь себя.** Какой станет вариантность `DmaWindow` по `'a`, если
заменить маркер на `PhantomData<Cell<&'a mut [u8]>>`? Скомпилируется ли
тогда `fn shorten<'a>(w: DmaWindow<'static>) -> DmaWindow<'a> { w }`?

</div>
