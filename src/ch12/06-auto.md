# 12.6. Что замыкание наследует от захваченного

## `Copy` и `Clone`

Структуру замыкания объявляет компилятор, и `derive` у неё нет. Вместо
этого действует правило: замыкание реализует `Copy`, если `Copy` всё,
что оно захватило, и `Clone`, если `Clone` всё захваченное. Захват по
общей ссылке — поле `&T`, а общая ссылка всегда `Copy`, поэтому
замыкание, которое только читает, копируется всегда:

```rust
{{#include ../../code/ch12/src/auto.rs:copy}}
```

`copies()` возвращает `(9, 9)`: и копия, и оригинал остались рабочими.
Изменяемая ссылка не `Copy` — две копии означали бы два `&mut` на одну
переменную, — и замыкание, которое меняет захваченное по ссылке, не
копируется:

```console
error[E0277]: the trait bound `&mut u32: Copy` is not satisfied in `{closure@w1.rs:3:16: 3:18}`
3 | let mut bump = || n += 1;
  |                -- within this `{closure@w1.rs:3:16: 3:18}`
4 | let mut other = copy(&bump);
  |                 ---- ^^^^^ within `{closure@w1.rs:3:16: 3:18}`, the trait `Copy` is not implemented for `&mut u32`
  |                 |
  |                 required by a bound introduced by this call
  = note: `Copy` is implemented for `&u32`, but not for `&mut u32`
note: required because it's used within this closure
```

Компилятор называет поле, из-за которого не вышло, и показывает
замыкание, в котором оно лежит. Копируемость важна на практике:
`Copy`-замыкание можно отдать в два [итератора][g-iterator] подряд или положить в
массив `[f; 4]`, не думая о владении.

## `Send` и `Sync`

`Send` и `Sync` — [автотрейты][g-auto-trait] (глава 9), и для замыкания они выводятся
так же, как для структуры с теми же полями. Отсюда правило, которое
стоит запомнить, потому что способ захвата выбирает компилятор:

| Захват | Поле | Замыкание `Send`, если |
|---|---|---|
| по ссылке | `&T` | `T: Sync` |
| по изменяемой ссылке | `&mut T` | `T: Send` |
| по значению | `T` | `T: Send` |

Один и тот же `Cell` даёт разный ответ в зависимости от того, как его
захватили:

```rust
{{#include ../../code/ch12/src/auto.rs:send}}
```

`Cell` можно передать в другой поток целиком, но нельзя делить между
потоками: он меняет значение без синхронизации. `owned` забрал `Cell` по
значению и потому `Send`; `borrowed` держит `&Cell`, а для `Send`
ссылки нужен `Cell: Sync`:

```console
error[E0277]: `Cell<u32>` cannot be shared between threads safely
4 | is_send(&borrowed);
  | ------- ^^^^^^^^^ `Cell<u32>` cannot be shared between threads safely
  | |
  | required by a bound introduced by this call
  = help: the trait `Sync` is not implemented for `Cell<u32>`
  = note: required for `&Cell<u32>` to implement `Send`
note: required because it's used within this closure
3 | let borrowed = || cell.get();
  |                ^^
```

`Rc` не `Send` ни в каком виде — его счётчик ссылок меняется без
атомарных операций, — и `thread::spawn` не примет замыкание, которое
его захватило:

```console
error[E0277]: `Rc<u32>` cannot be sent between threads safely
3 | let h = std::thread::spawn(move || *rc + 1);
  |         ------------------ -------^^^^^^^^
  |         |                  |
  |         |                  `Rc<u32>` cannot be sent between threads safely
  |         |                  within this `{closure@w3.rs:3:28: 3:35}`
  |         required by a bound introduced by this call
  = help: within `{closure@w3.rs:3:28: 3:35}`, the trait `Send` is not implemented for `Rc<u32>`
note: required because it's used within this closure
note: required by a bound in `spawn`
```

Этой защиты у `void *ctx` из введения нет вовсе: здесь передать
контекст в другой поток, не убедившись, что он к этому готов, нельзя.
Потоки, `Send` и `Sync` подробно разобраны в главе 20.

## Автотрейты проходят сквозь `impl Trait`

`impl FnMut() -> u32` скрывает тип замыкания, но не его автотрейты.
Если скрытый тип `Send`, вызывающий может на это положиться, хотя в
сигнатуре `Send` не написан:

```rust
{{#include ../../code/ch12/src/auto.rs:leak}}
```

С обычными трейтами так не бывает. Замыкание из `pipeline` в разделе
[12.4](04-return.md) захватывает два `i32` и само по себе `Copy`, но
снаружи видно только `impl Fn(i32) -> i32`:

```console
error[E0277]: the trait bound `impl Fn(i32) -> i32: Copy` is not satisfied
3 | let q = copy(&p);
  |         ---- ^^ the trait `Copy` is not implemented for `impl Fn(i32) -> i32`
  |         |
  |         required by a bound introduced by this call
```

Утечка автотрейтов удобна: не нужно писать `+ Send` в каждой
сигнатуре. Но она же делает `Send` частью интерфейса, о которой автор
функции может не знать. Добавил в замыкание `Rc` — и ошибка появится у
пользователя функции, в чужом крейте, далеко от правки. Задача D этой
главы разбирает именно такой случай и способ вернуть ошибку туда, где
её причина.

## Асинхронные замыкания

`async` перед замыканием делает его асинхронным: вызов возвращает
future, который нужно дождаться через `.await`. Опрашивать future будем
простейшим [исполнителем][g-executor], который крутит `poll`, пока не будет готово:

```rust
{{#include ../../code/ch12/src/auto.rs:block-on}}
```

<div class="box ref">

**Справка: future и `poll`.** Future — значение, которое можно
*опрашивать*: метод `poll` либо возвращает `Poll::Ready(результат)`,
либо `Poll::Pending` и тогда обещает разбудить исполнителя через
[`Waker`][g-waker] из `Context`, когда появится смысл опросить снова. `async fn`
и `async`-блок компилятор превращает в машину состояний, которая
реализует `Future`. `Waker::noop()` (Rust 1.85) — будильник, который
ничего не делает: годится, когда future никогда не ждёт внешнего
события. Как устроены машины состояний и настоящие исполнители — глава
22.

</div>

Асинхронные замыкания и трейты `AsyncFn`, `AsyncFnMut`, `AsyncFnOnce`
появились в Rust 1.85. Их главное отличие от «замыкания, которое
возвращает future» в том, что future может заимствовать состояние
самого замыкания:

```rust
{{#include ../../code/ch12/src/auto.rs:async-closure}}
```

`async_sum(&[1, 2, 3])` возвращает 6. Future, который вернул вызов
`f(b)`, держит `&mut` на структуру замыкания, а значит, и на захваченный
`sum`, и живёт, пока его не дождались. Следующий вызов возможен только
после этого — то, что `AsyncFnMut` и выражает.

До 1.85 то же писали через обычное замыкание, которое возвращает
`async`-блок, и граница выглядела как `F: FnMut(u8) -> Fut, Fut:
Future`. Future здесь — отдельное значение, которое не может
заимствовать у замыкания: по сигнатуре `FnMut` результат вызова не
связан с `&mut self`. Без `move` компилятор отказывает дважды:

```console
error[E0373]: async block may outlive the current function, but it borrows `b`, which is owned by the current function
error: captured variable cannot escape `FnMut` closure body
3 | let mut sum = 0u32;
  |     ------- variable defined here
4 | block_on(each_old(&[1, 2, 3], |b| async { sum += u32::from(b) }));
  |                                 - ^^^^^^^^---^^^^^^^^^^^^^^^^^^
  |                                 | |       |
  |                                 | |       variable captured here
  |                                 | returns an `async` block that contains a reference to a captured variable, which then escapes the closure body
  |                                 inferred to be a `FnMut` closure
  = note: `FnMut` closures only have access to their captured variables while they are executing...
  = note: ...therefore, they cannot allow references to captured variables to escape
```

А с `move` код компилируется и ошибается молча:

```rust
{{#include ../../code/ch12/src/auto.rs:async-old}}
```

`async_old_sum(&[1, 2, 3])` возвращает 0. `sum` — `u32`, то есть
`Copy`, и `async move` копирует его в каждый future: каждый future
прибавляет байт к своей копии и уничтожает её. Это ловушка `move` из
раздела [12.1](01-capture.md), только копий три, по одной на вызов.
Здесь rustc хотя бы предупреждает (в коде предупреждение подавлено
атрибутом, чтобы пример собирался без шума):

```console
warning: value captured by `sum` is never read
4 | block_on(each_old(&[1, 2, 3], |b| async move { sum += u32::from(b) }));
  |                                                ^^^
  = help: did you mean to capture by reference instead?
```

<div class="box trap">

**Ловушка.** Асинхронное замыкание тоже бывает `move`, и тогда `sum`
переезжает в структуру замыкания, а каждый future прибавляет к этой
копии:

```rust
{{#include ../../code/ch12/src/auto.rs:async-move}}
```

`async_move_sum(&[1, 2, 3])` возвращает 0, и здесь не предупреждает ни
rustc 1.97, ни clippy, в том числе с группами `pedantic` и `nursery`:
присваивание читается следующим вызовом, переменная `sum` объявлена
`mut` не зря. Если результат нужен снаружи, `move` не пишут или
захватывают ссылку явно, как в разделе 12.1.

</div>

<div class="box check">

**Проверь себя.** Будет ли `Send` замыкание `move || v.len()`, где `v:
Vec<Rc<u8>>`? А `|| v.len()` с тем же `v`? Почему замыкание из
`make_counter` можно передать в `thread::spawn`, если в сигнатуре
`make_counter` нет `Send`, и что сломается у вызывающего, если внутрь
счётчика добавить `Rc`? `each` принимает и обычное замыкание, которое
возвращает `async move`-блок: стандартная библиотека реализует
`AsyncFnMut` для таких замыканий. Что тогда окажется в `sum` и
предупредит ли об этом rustc?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-auto-trait]: ../appendix-f.md#auto-trait "Автотрейт (auto trait): Маркерный трейт, который компилятор реализует сам для каждого типа, все поля которого его реализуют: Send, Sync, Unpin, UnwindSafe."
[g-executor]: ../appendix-f.md#executor "Исполнитель (executor): Часть асинхронного рантайма, которая держит очередь готовых задач и опрашивает их, пока они не вернут Ready."
[g-iterator]: ../appendix-f.md#iterator "Итератор (iterator): Тип, реализующий трейт Iterator: состояние обхода с методом next, который выдаёт следующий элемент или None."
[g-waker]: ../appendix-f.md#waker "Будильник (waker): Значение типа Waker из Context, через которое future просит исполнителя опросить её снова."
