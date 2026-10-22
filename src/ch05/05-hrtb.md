# 5.5. Ограничения для всех времён жизни: `for<'a>`

## Время жизни, которое нельзя назвать

Функция суммирует значения из строк конфигурации. Строки с
экранированием (`r\x=5`) она сначала переписывает в локальный буфер и
отдаёт замыканию `f` уже буфер:

```rust
{{#include ../../code/ch05/src/hrtb.rs:total}}
```

Какое время жизни у аргумента `f`? Иногда это кусок `text`, иногда —
`buf`, который живёт только внутри `total_after` и переписывается на
каждой итерации. Параметр функции `'x` не подходит: параметры выбирает
вызывающий, и они длятся весь вызов. Если всё же написать `F:
FnMut(&'x str)`, компилятор потребует, чтобы `buf` был заимствован на
весь `'x`:

```text
error[E0597]: `buf` does not live long enough
  --> hrtb_0.rs:14:14
   |
 5 | pub fn total_named<'x, F>(text: &'x str, mut f: F) -> u32
   |                    -- lifetime `'x` defined here
...
 9 |     let mut buf = String::new();
   |         ------- binding `buf` declared here
...
14 |             &buf[..]
   |              ^^^ borrowed value does not live long enough
...
18 |         sum += f(line).unwrap_or(0);
   |                ------- argument requires that `buf` is borrowed for `'x`
...
21 | }
   | - `buf` dropped here while still borrowed
```

Вместе с ней приходит E0502: на следующей итерации `unescape` берёт
`&mut buf`, а ссылка из прошлой итерации по контракту ещё жива.

`for<'a> FnMut(&'a str) -> Option<u32>` говорит другое: *для любого*
`'a` замыкание принимает `&'a str`. Время жизни выбирает функция при
каждом вызове `f`, и оно может быть сколь угодно коротким. Такое
ограничение называется HRTB (higher-ranked trait bound).

Писать `for<'a>` у замыканий приходится редко. `FnMut(&str)` с
пропущенным временем жизни — сокращённая запись той же границы, и
`total_after_short` в примере компилируется так же.

## Что `for<'a>` запрещает замыканию

Раз ссылка действительна только на время вызова, сохранить её снаружи
нельзя:

```rust,ignore
let mut saved = Vec::new();
total_after(&text, |l| {
    saved.push(l);
    None
});
```

```text
error[E0521]: borrowed data escapes outside of closure
 --> hrtb_2.rs:8:5
  |
6 | let mut saved = Vec::new();
  |     --------- `saved` declared here, outside of the closure body
7 | total_after(&text, |l| {
  |                     - `l` is a reference that is only valid in the closure body
8 |     saved.push(l);
  |     ^^^^^^^^^^^^^ `l` escapes the closure body here
```

Ошибка спасает от настоящей беды: строки из `buf` перезаписываются на
следующей итерации, и сохранённая ссылка показывала бы чужие данные —
ровно случай `config.c` из введения.

<div class="box c">

**Как в C.** В C это контракт из документации к обратному вызову:
«указатель действителен только во время вызова». Так устроены `qsort`
(сравнивает элементы, которые потом переставит), `nftw` (путь к файлу
собирается во внутреннем буфере) и `dl_iterate_phdr`. Нарушение никто не
проверяет. `for<'a>` — тот же контракт в типе.

</div>

## `for<'a>` без замыканий

Сокращённая запись есть только у `Fn`, `FnMut` и `FnOnce`. Для остальных
трейтов граница с ссылкой внутри записывается явно:

```rust
{{#include ../../code/ch05/src/hrtb.rs:take_sum}}
```

`take_sum` складывает отсчёты в буфере любого типа, который можно обойти
по ссылке, — `Vec<u32>`, массива, `BTreeSet<u32>` — и очищает буфер.
Ссылка `&*buf` для обхода живёт одну строку. Если взять её время жизни из
параметров (`buf: &'x mut C` и `&'x C: IntoIterator`), обход заимствует
буфер на весь вызов, и очистка не компилируется:

```text
error[E0506]: cannot assign to `*buf` because it is borrowed
 --> hrtb_1.rs:9:5
  |
3 | pub fn take_sum_named<'x, C>(buf: &'x mut C) -> u32
  |                       -- lifetime `'x` defined here
...
8 |     let sum = (&*buf).into_iter().sum();
  |               -------------------
  |               |
  |               `*buf` is borrowed here
  |               argument requires that `*buf` is borrowed for `'x`
9 |     *buf = C::default();
  |     ^^^^ `*buf` is assigned to here but it was already borrowed
```

С `for<'a>` компилятор выбирает для этой ссылки своё короткое время
жизни, как для аргумента замыкания в `total_after`.

## Замыкание, которое возвращает ссылку

Обратная сторона: замыкание, которое отдаёт часть аргумента, само по себе
не компилируется.

```text
error: lifetime may not live long enough
 --> hrtb_3.rs:3:30
  |
3 | let skip_header = |x: &[u8]| &x[1..];
  |                       -    - ^^^^^^^ returning this value requires that `'1` must outlive `'2`
  |                       |    |
  |                       |    return type of closure is &'2 [u8]
  |                       let's call the lifetime of this reference `'1`
```

У функций правило элизии 2 связало бы вход с выходом. У замыканий
правил элизии нет: компилятор выводит сигнатуру по телу и для
возвращаемой ссылки заводит отдельное время жизни `'2`. Аннотация `->
&[u8]` не помогает — тоже `'1` и `'2`. Синтаксис `for<'a> |x: &'a [u8]|
-> &'a [u8]` в rustc 1.97 нестабилен (E0658, issue 97362). Работают два
способа: написать обычную функцию или передать замыкание в функцию с
нужной границей — она задаёт сигнатуру:

```rust
{{#include ../../code/ch05/src/hrtb.rs:slicer}}
```

```rust,ignore
let skip_header = slicer(|x| &x[1..]);
```

Тип указателя на функцию с пропущенным временем жизни — тоже
`for<'a>`: `fn(&[u8]) -> &[u8]` значит `for<'a> fn(&'a [u8]) -> &'a [u8]`.
Замыкание без захвата приводится к такому указателю, и тест
`closure_signature` это проверяет. `type_name` печатает этот тип как
`fn(&'_ [u8]) -> &'_ [u8]`.

<div class="box hood">

**Под капотом.** `for<'a>` ничего не добавляет в машинный код: после
проверки времена жизни стираются (раздел [5.3](03-static.md)), и
`total_after` мономорфизируется по типу замыкания, как любая обобщённая
функция (глава [10](../ch10.md)). Проверка же при вызове обобщённой
функции идёт один раз — для всех `'a` сразу, по сигнатуре замыкания.

</div>

<div class="box check">

**Проверь себя.** Функция только суммирует и буфер не очищает:
`fn sum<'a, C>(c: &'a C) -> u32 where &'a C: IntoIterator<Item = &'a
u32>`. Нужен ли здесь `for<'a>`? Чем она отличается от `take_sum`?

</div>
