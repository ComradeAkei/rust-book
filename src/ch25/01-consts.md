# 25.1. const и static

## Значение и объект

В Rust два вида элементов, которые вычисляются при компиляции, и
разница между ними — та же, что между `#define` и глобальной
переменной в C:

```rust
{{#include ../../code/ch25/src/consts.rs:items}}
```

`const` — **значение**. У него нет своего места в памяти: компилятор
подставляет его в каждое место использования. Поэтому константа годится
везде, где нужно значение, известное при компиляции: в длину массива
(`[u8; HEADER_LEN]` — тип), в образец `match` (ветка `HEADER_LEN`
сравнивает с 8, а не вводит новое имя), в обычное выражение.

`static` — **объект**. Он один на всю программу, у него есть адрес, и
каждое обращение к нему читает память по этому адресу. Это глобальная
переменная C, только неизменяемая по умолчанию.

Если нужно назвать число 8 для длины заголовка, выбираем `const`:
важно значение. Если нужен один общий счётчик принятых кадров,
выбираем `static` с подходящим типом для синхронизации: важно, что
все обращения относятся к одному объекту. Это различие относится
к правилам языка; оптимизатор вправе убрать ненужное чтение памяти,
когда его результат заранее известен.

У этих двух ролей разные правила. Константа в образце — значение, а
`static` в образце — ошибка, а не новое имя:

```text
error[E0530]: match bindings cannot shadow statics
 --> e0530.rs:5:9
  |
1 | pub static OFFSET: u16 = 5;
  | --------------------------- the static `OFFSET` is defined here
...
5 |         OFFSET => true,
  |         ^^^^^^ cannot be named the same as a static
```

Без этой ошибки `OFFSET` в ветке стал бы новой переменной, которая
совпадает с чем угодно, — та же ловушка, что с константой, которую
забыли импортировать (раздел [2.3](../ch02/03-match.md)).

Объект `static` видят все потоки сразу, поэтому его тип обязан быть
[`Sync`][g-send-sync]:

```text
error[E0277]: `Cell<u32>` cannot be shared between threads safely
 --> cell.rs:3:18
  |
3 | pub static LAST: Cell<u32> = Cell::new(0);
  |                  ^^^^^^^^^ `Cell<u32>` cannot be shared between threads safely
  |
  = help: the trait `Sync` is not implemented for `Cell<u32>`
  = note: shared static variables must have a type that implements `Sync`
```

На микроконтроллере с одним ядром потоков нет, но есть обработчики
прерываний, и для них `static` — то же самое, что для потоков: объект,
к которому обращаются из двух мест, не договариваясь. Что делать, когда
`static` всё-таки нужно менять, — раздел [25.4](04-statics.md).

С Rust 1.83 константа может читать неизменяемый `static` без внутренней
изменяемости: его значение тоже известно при компиляции.

```rust
{{#include ../../code/ch25/src/consts.rs:static-in-const}}
```

<div class="box c">

**Как в C.** В C `const` — квалификатор объекта, а не константа. `const
int N = 16;` — переменная, которую нельзя менять, и в размере массива
в области файла она запрещена:

```c
{{#include ../../code/ch25/c/const_size.c:const_size}}
```

```text
const_size.c:5:22: error: variably modified 'buf' at file scope
```

Константы-значения в C — это `#define`, перечисления и, с C23,
`constexpr`. В Rust одно слово для каждой роли: `const` — значение,
`static` — объект.

</div>

## Повтор в массиве

`[x; N]` копирует значение `x` в `N` элементов, поэтому `x` обязан быть
`Copy`. Ячейка драйвера с буфером внутри — не `Copy`, и это намеренно:
буфер не должен размножаться неявно.

```text
error[E0277]: the trait bound `Slot: Copy` is not satisfied
 --> slot.rs:6:6
  |
6 |     [Slot { used: false }; 4]
  |      ^^^^^^^^^^^^^^^^^^^^ the trait `Copy` is not implemented for `Slot`
  |
  = note: the `Copy` trait is required because this value will be copied for each element of the array
```

С константой то же выражение компилируется:

```rust
{{#include ../../code/ch25/src/consts.rs:fresh}}
```

Константа — не объект, который копируют, а выражение, которое
вычисляют заново: `[Slot::FREE; 4]` — четыре вычисления `Slot::FREE`,
и `Copy` не нужен. Так же работает блок `[const { Slot { used: false,
data: [0; 8] } }; 4]`. Это обычный способ получить массив мьютексов,
ячеек или `Option<Box<T>>` без `Copy`.

## Константа с внутренней изменяемостью

Из «каждое упоминание — новое вычисление» следует ловушка:

```rust
{{#include ../../code/ch25/src/consts.rs:trap}}
```

`COUNTER.fetch_add(1, …)` создаёт временный атомик со значением 0,
увеличивает его и выбрасывает. Следующее упоминание `COUNTER` — снова 0.
Тест `guaranteed_const_is_a_fresh_value` проверяет это поведение: оно
гарантировано языком. rustc 1.97 предупреждает прямо:

```text
warning: mutation of an interior mutable `const` item with call to `fetch_add`
 --> trap.rs:6:5
  |
6 |     COUNTER.fetch_add(1, Ordering::Relaxed)
  |     -------^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |     |
  |     `COUNTER` is a interior mutable `const` item of type `Atomic<u32>`
  |
  = note: each usage of a `const` item creates a new temporary
  = note: only the temporaries and never the original `const COUNTER` will be modified
help: for a shared instance of `COUNTER`, consider making it a `static` item instead
```

Предупреждение — не ошибка: константа атомика законна и полезна, именно
ею инициализируют массив атомиков, `[ZERO; 16]`. Ошибка — менять её.

<div class="box trap">

**Ловушка.** `const` с `Cell`, атомиком или мьютексом внутри — шаблон
для инициализации, а не общий объект. Если два места программы должны
видеть одно состояние — это `static`. [clippy][g-clippy] ловит и само объявление
(`declare_interior_mutable_const`), и заём (`borrow_interior_mutable_const`).

</div>

## Ссылки на константы

Ссылка на `static` — всегда ссылка на тот же объект. А ссылка на
константу?

```rust
{{#include ../../code/ch25/src/consts.rs:address}}
```

`&LIMITS` — ссылка на безымянную копию значения, которую компилятор кладёт
в `.rodata` и которая живёт всю программу: это **продвижение в `'static`**
(promotion). Сколько таких копий — одна на программу или своя на каждое
место — язык не обещает: Reference прямо говорит, что ссылки на одну
константу не обязаны совпадать. Тест `observed_const_references_shared`
фиксирует, что делает rustc 1.97: `limits_a()` и `limits_b()` возвращают
один и тот же адрес — компилятор хранит одинаковые значения без повторов.
А под Miri, который держится того, что обещает язык, адреса разные, и тест
там пропущен. Сравнивать такие адреса в программе нельзя: это наблюдение,
а не гарантия. Если адрес важен — например, его передают в DMA или
регистрируют в таблице, — нужен `static`.

Продвигается не всякое выражение. Литерал и арифметика над литералами —
да: `&(2 + 2)` можно вернуть как `&'static u32`. Вызов функции — нет,
даже если функция `const`:

```text
error[E0515]: cannot return reference to temporary value
 --> e0515.rs:6:5
  |
6 |     &square(4)
  |     ^---------
  |     ||
  |     |temporary value created here
  |     returns a reference to data owned by the current function
```

Вне *константного контекста* (const context: инициализатор `const` или
`static`, длина массива, блок `const { … }`) `square(4)` — обычный вызов
при выполнении: функция могла бы паниковать или зависеть от входа, и
компилятор не вправе тайно вычислять её заранее. Блок
`const { square(4) }` делает вызов константой явно, и ссылка на неё —
`'static`. Оба случая — doc-тесты модуля `consts`.

<div class="box check">

**Проверь себя.** Сколько объектов `AtomicU32` создаёт выражение
`[COUNTER; 4]`, а сколько — `[&SHARED; 4]`, где `SHARED` — `static`?
Какой адрес у `COUNTER`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-clippy]: ../appendix-f.md#clippy "clippy: Линтер из тулчейна Rust: несколько сотен линтов, от неидиоматичного кода до заведомых ошибок, разбитых на группы с уровнями allow, warn и deny."
[g-send-sync]: ../appendix-f.md#send-sync "Send, Sync: Автотрейты потокобезопасности."
