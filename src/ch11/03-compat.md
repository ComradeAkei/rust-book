# 11.3. Совместимость с dyn

## Таблица на все случаи

Таблицу методов компилятор строит один раз для пары «тип — трейт», и в
ней должен найтись адрес каждого метода, который можно вызвать у
объекта. Добавим в датчик АЦП метод, который пишет отсчёт в любой
приёмник `fmt::Write`:

```rust,ignore
pub trait Probe {
    fn read(&self) -> i32;
    fn log<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
        write!(w, "{}", self.read())
    }
}
```

Для обобщённого кода из главы 10 трейт годится: `log::<String>` и
`log::<Uart>` мономорфизуются по отдельности. А `dyn Probe` уже не
собирается:

```text
error[E0038]: the trait `Probe` is not dyn compatible
 --> e4.rs:8:19
  |
8 | pub fn report(p: &dyn Probe) -> i32 {
  |                   ^^^^^^^^^ `Probe` is not dyn compatible
  |
note: for a trait to be dyn compatible it needs to allow building a vtable
      for more information, visit <https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility>
 --> e4.rs:4:8
  |
2 | pub trait Probe {
  |           ----- this trait is not dyn compatible...
3 |     fn read(&self) -> i32;
4 |     fn log<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
  |        ^^^ ...because method `log` has generic type parameters
  = help: consider moving `log` to another trait
```

Копий `log` столько, сколько типов `W` встретится в программе, а все
они вместе неизвестны ни одному крейту: следующий крейт может вызвать
`log` со своим приёмником. В слот таблицы положить нечего. Ошибка
возникает там, где написано `dyn Probe`, а не у трейта: сам трейт
корректен, он просто не годится для объектов. Это свойство называется
*совместимостью с dyn* (dyn compatibility); до 2024 года его называли
object safety, и старое название живёт в книгах и обсуждениях.

Сравните два неизвестных типа. В `fn f<T: Sensor>(x: &T)` имя `T`
сохраняется: при создании копии функции компилятор подставит реальный
тип. В `fn f(x: &dyn Sensor)` у вызываемого кода такой подстановки
нет: ему передана ссылка с таблицей. Поэтому он должен иметь
достаточно сведений для каждого вызова прямо в этой ссылке;
ограничения ниже объясняют, когда это возможно.

## Правила

Компилятор проверяет все методы и ассоциированные элементы трейта и его
супертрейтов. Каждое правило — следствие того, что вызывающий знает
только таблицу:

| В трейте | Сообщение `…because` | Почему таблица не строится |
|---|---|---|
| `fn log<W: Write>(&self, w: &mut W)` | method `log` has generic type parameters | копия на каждый `W` |
| `fn fresh(&self) -> Self` | method `fresh` references the `Self` type in its return type | размер результата неизвестен вызывающему |
| `fn same(&self, other: &Self)` | method `same` references the `Self` type in this parameter | второй аргумент должен быть того же неизвестного типа, и проверить это нечем |
| `fn kind() -> u8` | associated function `kind` has no `self` parameter | нет объекта, из указателя на который взять таблицу |
| `const MAX: i32;` | it contains associated const `MAX` | слотов для констант в таблице нет |
| `trait Snapshot: Clone` | it requires `Self: Sized` | `dyn Snapshot` пришлось бы иметь размер |
| `fn samples(&self) -> impl Iterator` | method `samples` references an `impl Trait` type in its return type | у каждой реализации свой скрытый тип и свой размер |
| `async fn next(&self) -> i32` | method `next` is `async` | то же: будущее скрытого типа |

Обобщённым считается и `w: &mut impl Write` в аргументе (раздел
[10.3](../ch10/03-impl-trait.md)), а параметр времени жизни —
нет: `fn pick<'a>(&'a self, other: &'a [i16]) -> &'a i16` совместим,
времена жизни при компиляции стираются, и копия у метода одна. `Self`
за указателем в получателе — `&self`, `&mut self`, `self: Box<Self>`,
`self: Rc<Self>`, `self: Pin<&mut Self>` — разрешён: это и есть адрес
данных толстого указателя. Все восемь случаев проверяют doc-тесты в
`code/ch11/src/compat.rs` с кодом ошибки.

`self` по значению совместимости не мешает, но вызвать такой метод у
объекта нельзя — значение без размера не [переместить][g-move]:

```text
error[E0161]: cannot move a value of type `dyn Probe`
 --> e10.rs:6:38
  |
6 | pub fn f(p: Box<dyn Probe>) -> i32 { p.finish() }
  |                                      ^ the size of `dyn Probe` cannot be statically determined
```

`fn finish(self: Box<Self>)` вызывается: метод получает `Box`, то есть
адрес, а память освобождает сам.

## where Self: Sized

Для функции без `self` компилятор подсказывает два выхода:

```text
help: consider turning `kind` into a method by giving it a `&self` argument
  |
3 |     fn kind(&self) -> u8;
  |             +++++
help: alternatively, consider constraining `kind` so it does not apply to trait objects
  |
3 |     fn kind() -> u8 where Self: Sized;
  |                     +++++++++++++++++
```

Второй работает для любого метода из таблицы правил. `dyn Probe` не
[`Sized`][g-sized], поэтому метод с `where Self: Sized` у объекта не существует,
в таблицу не попадает и перестаёт мешать. Трейт датчика АЦП из
`code/ch11/src/compat.rs`:

```rust
{{#include ../../code/ch11/src/compat.rs:probe}}
```

```rust
{{#include ../../code/ch11/src/compat.rs:probe-impl}}
```

У `Adc` есть все четыре метода, и `log_adc` мономорфизует `log` для
своего приёмника. `zero` без тела, и реализация обязана его написать,
хотя через объект он недоступен. У `&dyn Probe` остаются `read` и
`log_dyn`, и таблица это подтверждает:

```console
$ cargo asm -p ch11 --lib --include-constants ch11_as_probe
…
.Lanon.da1beaeebe5f420a6faa69c57adbc3f3.16:
	.asciz	"\000\000\000\000\000\000\000\000\002\000\000\000\000\000\000\000\002\000\000\000\000\000\000"
	.quad	<ch11::compat::Adc as ch11::compat::Probe>::read
	.quad	<ch11::compat::Adc as ch11::compat::Probe>::log_dyn
```

Пять слов: деструктора нет, размер и выравнивание 2, затем `read` и
`log_dyn`. Ни `log`, ни `zero` в таблице нет, хотя в трейте `log`
объявлен вторым. Вызвать их у объекта компилятор не даст:

```text
error: the `log` method cannot be invoked on a trait object
  --> e7.rs:12:7
   |
 6 |         Self: Sized,
   |               ----- this has a `Sized` requirement
...
12 |     p.log(out)
   |       ^^^
```

`log_dyn` — тот же `log`, у которого параметр типа заменён объектом:
`&mut dyn fmt::Write`. Копия у метода одна, приёмник выбирается во
время работы через его собственную таблицу. Так устроен и
`core::fmt`: `Formatter` держит приёмник как `&mut dyn Write`, а
каждый аргумент внутри `Arguments` хранится парой «адрес значения и
адрес функции форматирования» — объектом трейта, собранным вручную.
Поэтому код форматирования один на все типы приёмников.

<div class="box hood">

**Под капотом.** В `ch11_log_dyn` встречаются два толстых указателя:

```console
$ cargo asm -p ch11 --lib --simplify ch11_log_dyn
ch11_log_dyn:
	mov rax, qword ptr [rsi + 32]
	mov rsi, rdx
	mov rdx, rcx
	jmp rax
```

`p` приходит в `rdi:rsi`, `w` — в `rdx:rcx`. Слот 4 таблицы `p` —
`log_dyn`, его аргументы — `&self` в `rdi` и `w` в `rsi:rdx`, поэтому
пара `w` сдвигается на регистр влево, и остаётся переход. Таблица `w`
не читается: её прочтёт `log_dyn`, когда начнёт писать.

</div>

Компилятор сам реализует трейт для объекта: `dyn Probe: Probe`, и
обобщённая функция с `T: Probe + ?Sized` примет `dyn Probe` (раздел
[11.4](04-objects.md)). Написать такую реализацию вручную нельзя:

```text
error[E0371]: the object type `(dyn P + 'static)` automatically implements the trait `P`
```

Метод с `where Self: Sized` эта реализация не включает, и требовать его
от `dyn Probe` не приходится. Стандартная библиотека пользуется приёмом
повсюду: у `Iterator` один обязательный метод `next` и десятки
обобщённых — `map`, `filter`, `collect`, — и все они с `where Self:
Sized`. Поэтому `&mut dyn Iterator<Item = i16>` возможен, а `map` у
него вызывается через пересылающую реализацию для `&mut I` из раздела
[11.4](04-objects.md).

<div class="box c">

**Как в C.** Обобщённый метод в C-таблице выглядит как пара «функция
обратного вызова и контекст»:

```c
int (*log)(const struct dev *d, int (*emit)(void *ctx, const char *s), void *ctx);
```

`void *ctx` и `emit` — те же два слова, что у `&mut dyn fmt::Write`,
только связывает их программист, и `ctx` теряет тип. В C++ шаблонный
метод нельзя сделать виртуальным по той же причине, что и здесь: g++
отвечает `templates may not be 'virtual'`, clang — `'virtual' cannot be
specified on member function templates`.

</div>

## Копия объекта

`Clone` несовместим: `clone` возвращает `Self`, и сам трейт требует
`Sized`. Но копию объекта можно вернуть за указателем — у `Box<dyn
Trait>` размер известен:

```rust
{{#include ../../code/ch11/src/compat.rs:clone-box}}
```

Обобщённая реализация даёт `clone_box` каждому `Sensor + Clone`, и
писать её для каждого датчика не нужно. `impl Clone for Box<dyn
SensorBox>` замыкает круг: `Vec<Box<dyn SensorBox>>` клонируется
обычным `.clone()`, и каждый элемент копируется через свою таблицу.
`'static` в реализации нужен потому, что `Box<dyn SensorBox>` означает
`Box<dyn SensorBox + 'static>` (раздел [11.1](01-dyn.md)): тип, который
держит заём, туда не положить. Тест `boxed_clones` копирует вектор из
`Thermo` и `Counter` и проверяет, что у копий те же отсчёты и имена.

<div class="box trap">

**Ловушка.** Обобщённый метод делает трейт несовместимым с `dyn`, даже
если у метода есть тело по умолчанию и никто его не переопределяет.
Добавить такой метод в публичный трейт без `where Self: Sized` — ломающее
изменение: у всех, кто писал `dyn Trait`, код перестанет собираться, а
у тех, кто пользуется только обобщениями, — нет, и тесты самого крейта
могут этого не заметить. Если трейт задуман для объектов, проверку
стоит закрепить в самом крейте, как в `code/ch11/src/compat.rs`:

```rust
{{#include ../../code/ch11/src/compat.rs:guard}}
```

</div>

<div class="box check">

**Проверь себя.** Совместим ли с `dyn` трейт с методом `fn merge(&self,
other: &dyn Probe) -> i32`? А с методом `fn cmp(&self, other: &Self)
where Self: Sized`? Почему `Iterator::next` попадает в таблицу, а
`Iterator::map` — нет, и что из этого следует для `Box<dyn Iterator<Item
= u8>>`? Сколько слов займёт таблица `dyn Probe`, если добавить в трейт
`fn reset(&mut self)`, а сколько — если ещё и `fn scaled<const K: i32>(&self)
-> i32 where Self: Sized`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-move]: ../appendix-f.md#move "Перемещение (move): Передача владения значением: после let b = a владеет b, а a больше использовать нельзя."
[g-sized]: ../appendix-f.md#sized "Sized, ?Sized: Sized — маркерный трейт типов, размер которых известен при компиляции; компилятор неявно требует его от каждого параметра типа."
