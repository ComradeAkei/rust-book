# 9.7. Приёмы и ловушки

## Запечатанный трейт

Блоки периферии микроконтроллера известны заранее: у кристалла два UART,
и третий пользователь крейта не добавит. Трейт, который их описывает,
должен быть доступен как ограничение, но закрыт для реализации снаружи:

```rust
{{#include ../../code/ch09/src/tricks.rs:sealed}}
```

`Peripheral` публичен, но требует супертрейт `private::Sealed`, а модуль
`private` виден только внутри крейта. Пользователь может написать
`fn base<P: Peripheral>() -> usize { P::BASE }` и вызвать её для `Uart0`,
но не может реализовать `Peripheral` для своего типа: для этого нужен
`Sealed`, а назвать его нельзя. rustc 1.97 узнаёт этот приём и
объясняет ошибку:

```text
error[E0277]: the trait bound `MyUart: tricks::private::Sealed` is not satisfied
 --> e0277seal.rs:3:21
  |
3 | impl Peripheral for MyUart {
  |                     ^^^^^^ unsatisfied trait bound
…
  = note: `Peripheral` is a "sealed trait", because to implement it you also need to implement `ch09::tricks::private::Sealed`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
  = help: the following types implement the trait:
            ch09::tricks::Uart0
            ch09::tricks::Uart1
```

Попытка реализовать сам `Sealed` по полному пути упирается в
видимость модуля:

```text
error[E0603]: module `private` is private
 --> e0603.rs:2:20
  |
2 | impl ch09::tricks::private::Sealed for MyUart {}
  |                    ^^^^^^^  ------ trait `Sealed` is not publicly re-exported
  |                    |
  |                    private module
```

Печать даёт крейту три вещи:

- Закрытый список. Небезопасный код крейта может полагаться на то, что
  `P::BASE` — адрес настоящего UART, не делая сам трейт `unsafe`:
  чужих реализаций с неверным адресом не бывает.
- Свободу развития. В трейт можно добавить обязательный метод или
  константу, и это не сломает пользователей: реализации пишет только сам
  крейт, и он обновит их в той же версии.
- Понятную документацию: rustdoc и сообщение компилятора перечисляют
  все реализации.

Так устроен `embedded-hal` 1.0: `i2c::AddressMode`, параметр трейта
`I2c`, запечатан, и реализуют его только `SevenBitAddress` (`u8`) и
`TenBitAddress` (`u16`). Драйверу этого достаточно, чтобы знать все
возможные виды адресов. Задача [9.C](08-tasks.md) запечатывает выводы
GPIO.

## Трейт-расширение

Добавить метод чужому типу напрямую нельзя:

```text
error[E0390]: cannot define inherent `impl` for primitive types
 --> e0390.rs:1:1
  |
1 | impl [u8] { pub fn checksum(&self) -> u8 { 0 } }
  | ^^^^^^^^^
  |
  = help: consider using an extension trait instead
```

Компилятор сам подсказывает выход: свой трейт для чужого типа правило
сироты разрешает.

```rust
{{#include ../../code/ch09/src/tricks.rs:extension}}
```

Теперь у каждого `[u8]` есть `checksum`, но только там, где трейт
импортирован. Без `use ch09::tricks::SliceExt;` метод не виден, и
компилятор подсказывает, что импортировать:

```text
error[E0599]: no method named `checksum` found for array `[u8; 3]` in the current scope
 --> e0599b.rs:2:51
  |
2 | pub fn f() -> u8 { let frame = [1u8, 2, 3]; frame.checksum() + Uart0::BASE as u8 }
  |                                                   ^^^^^^^^ method not found in `[u8; 3]`
…
  = help: items from traits can only be used if the trait is in scope
help: trait `SliceExt` which provides `checksum` is implemented but not in scope; perhaps you want to import it
  |
1 + use ch09::tricks::SliceExt;
  |
```

В экосистеме встраиваемого Rust расширения встречаются постоянно:
крейт `fugit` добавляет целым числам методы `.millis()` и `.micros()`
через трейт `ExtU32`, а HAL-крейты дают регистрам периферии метод
`.split()` через трейты-расширения. Задача [9.C](08-tasks.md) добавляет
`u32` методы для битовых полей.

<div class="box trap">

**Ловушка.** Собственный метод типа всегда побеждает метод трейта с тем
же именем. Если в будущей версии `core` у срезов появится свой
`checksum`, вызов `frame.checksum()` молча переключится на него, хотя
ваш код не менялся. Пока такой метод нестабилен, rustc предупреждает
линтом `unstable_name_collisions`, и это время, чтобы переименовать свой
метод или перейти на полную запись `SliceExt::checksum(&frame)`.

</div>

## Deref вместо наследования

Драйверу UART нужны методы блока регистров, и хочется получить их
«даром», как от базового класса:

```rust
{{#include ../../code/ch09/src/tricks.rs:deref}}
```

`Deref` позволяет `u.enable()`: метод не найден у `Uart`, и компилятор
ищет его у `*u`, то есть у `Regs`. Выглядит как наследование, но ведёт
себя иначе. `Uart` не реализует `Clone`, а `u.clone()` компилируется —
и возвращает `Regs`:

```text
error[E0308]: mismatched types
 --> e0308.rs:2:78
  |
2 | pub fn f() { let u = Uart::new(Regs { base: 0x4000_C000 }); let copy: Uart = u.clone(); }
  |                                                                       ----   ^^^^^^^^^ expected `Uart`, found `Regs`
  |                                                                       |
  |                                                                       expected due to this
```

Без аннотации типа ошибки не было бы вовсе: `let copy = u.clone();`
тихо создаёт копию регистров без `baud`, и следующая строка, которая
ждёт драйвер, сломается далеко от места причины. То же со сравнением,
печатью и любым методом, которого нет у обёртки. Переопределения тоже
нет: если `Uart` объявит свой `enable`, он закроет метод `Regs` для
вызова через точку, но функция, принимающая `&Regs`, получит `&Uart`
через неявное приведение и вызовет метод `Regs`, о котором драйвер
ничего не знает.

`Deref` предназначен для умных указателей: `Box<T>` к `T`, `String` к
`str`, `Vec<T>` к `[T]`. Там «вести себя как содержимое» — именно то,
что нужно. Для общего поведения нескольких типов в Rust есть трейты с
методами по умолчанию, а для повторного использования данных — явное
поле и короткие методы-делегаты:

```rust
impl Uart {
    pub fn regs(&self) -> &Regs { &self.regs }
}
```

<div class="box c">

**Как в C.** `Deref` ближе всего к приёму с базовой структурой первым
полем из раздела [9.3](03-supertraits.md): производное можно подать
туда, где ждут базовое. Только в C приведение пишут руками и видят его в
коде, а `Deref` вставляет его сам, в том числе там, где его не ждали.

</div>

<div class="box check">

**Проверь себя.** Зачем `Sealed` объявлен `pub`, если модуль `private`
всё равно закрыт? Что случится с запечатыванием, если написать `pub mod
private`? Может ли другой крейт написать `impl SliceExt for Vec<u8>`? А
`impl SliceExt` для своего типа, и что изменилось бы, будь `SliceExt`
запечатан?

</div>
