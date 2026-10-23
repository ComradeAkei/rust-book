# 10.7. Типы-маркеры и typestate

## Маркер без значений

Контроллер DMA видит физическую память, процессор с MMU — виртуальную.
Оба адреса — числа одной ширины, и в C их путают постоянно:
`dma_start(buf, len)` с виртуальным адресом буфера собирается и пишет
мимо. Параметр типа может различать их без единого лишнего байта:

```rust
{{#include ../../code/ch10/src/typestate.rs:addr}}
```

`Phys` и `Virt` — перечисления без вариантов: значений у них нет вовсе,
создать их нельзя, и годятся они только на роль параметра. *Тип-маркер*
различает `Addr<Phys>` и `Addr<Virt>` при компиляции, а внутри обоих
один `usize`. Поле `PhantomData<S>` нужно, потому что параметр, который
нигде не использован, — ошибка E0392: компилятор должен знать, как `S`
влияет на вариантность и `Send`/`Sync` (раздел
[5.4](../ch05/04-variance.md)). Размер `PhantomData` — ноль.

```text
error[E0308]: mismatched types
 --> ts0308.rs:5:15
  |
5 |     dma_start(buf, 64);
  |     --------- ^^^ expected `Addr<Phys>`, found `Addr<Virt>`
  |     |
  |     arguments to this function are incorrect
  |
  = note: expected struct `Addr<Phys>`
             found struct `Addr<Virt>`
```

Получить `Addr<Phys>` из виртуального адреса можно только через
`Window::translate`, которая проверяет, что адрес попал в окно. Ошибка
перестала быть возможной, а не стала обнаруживаемой.

`Clone` и `Copy` для `Addr<S>` написаны руками, и это не случайность.
`#[derive(Clone, Copy)]` добавил бы к реализации ограничение `S: Copy`,
а у `Phys` и `Virt` его нет: `Addr<Phys>` копировать бы не удалось. Derive
ставит ограничения на все параметры типа, даже если в полях они стоят
только внутри `PhantomData`.

## Состояние в типе

Тот же приём для протокола: UART PL011 из раздела
[6.9](../ch06/09-registers.md) можно настроить только выключенным, а
передавать байты — только включённым. В C это правило живёт в
документации. В Rust состояние становится параметром типа драйвера:

```rust
{{#include ../../code/ch10/src/typestate.rs:states}}
```

Трейт `State` запечатан по образцу раздела [9.7](../ch09/07-tricks.md):
состояний ровно два, и третьего снаружи не добавить. Переходы — методы,
которые забирают драйвер по значению и возвращают его в новом типе:

```rust
{{#include ../../code/ch10/src/typestate.rs:transitions}}
```

У `Uart<Disabled>` есть `enable`, у `Uart<Enabled>` — `write_byte`,
`disable` и `fmt::Write`, а `release` есть у обоих. Компилятор
отвергает три вида ошибок. Запись через выключенный UART:

```text
error[E0599]: no method named `write_byte` found for struct `Uart<'_, ch10::typestate::Disabled>` in the current scope
 --> ts0599.rs:6:10
  |
6 |     uart.write_byte(b'A');
  |          ^^^^^^^^^^ method not found in `Uart<'_, ch10::typestate::Disabled>`
  |
  = note: the method was found for `Uart<'a, Enabled>`
```

Использование старого состояния после перехода — `enable(self)` забрал
значение, и прежнего `Uart<Disabled>` больше нет:

```text
error[E0382]: use of moved value: `off`
 --> ts0382.rs:7:17
  |
5 |     let off = Uart::new(regs);
  |         --- move occurs because `off` has type `Uart<'_, ch10::typestate::Disabled>`, which does not implement the `Copy` trait
6 |     let on = off.enable(BAUD_115200_12MHZ);
  |                  ------------------------- `off` moved due to this method call
7 |     let again = off.enable(BAUD_115200_12MHZ);
  |                 ^^^ value used here after move
```

И собственное состояние снаружи:

```text
error[E0277]: the trait bound `Broken: typestate::sealed::Sealed` is not satisfied
 --> ts0277.rs:3:33
  |
3 | impl ch10::typestate::State for Broken {}
  |                                 ^^^^^^ unsatisfied trait bound
…
  = note: `State` is a "sealed trait", because to implement it you also need to implement `ch10::typestate::sealed::Sealed`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
```

Это и есть *typestate*: состояние объекта — часть его типа, переход
меняет тип, а операция, недопустимая в состоянии, — метод, которого у
этого типа нет. Без владения и перемещения из главы 3 приём не работал
бы: ссылка на старое состояние осталась бы жить.

## Сколько это стоит

Во время работы от состояния не остаётся ничего: `Uart<'_, S>` — одна
ссылка на регистры. Для сравнения — тот же сценарий без типов, голыми
записями в регистры, и вариант с флагом, который проверяется во время
работы:

```rust
{{#include ../../code/ch10/src/typestate.rs:checked}}
```

```rust
{{#include ../../code/ch10/src/typestate.rs:cost}}
```

Сборка для Cortex-M3 и таблица символов:

```console
$ cargo build --release --lib -p ch10 --no-default-features --target thumbv7m-none-eabi
$ readelf -sW target/thumbv7m-none-eabi/release/libch10.rlib 2>/dev/null | grep -E 'FUNC.*ch10_(typed|raw)_send|FUNC.*_write$'
    82: 00000001    20 FUNC    GLOBAL DEFAULT   50 ch10_checked_write
    83: 00000001    76 FUNC    GLOBAL DEFAULT   54 ch10_raw_send
    91: 00000001    14 FUNC    GLOBAL DEFAULT   86 ch10_typed_write
    97: 00000001    76 FUNC    GLOBAL DEFAULT   54 ch10_typed_send
```

`ch10_typed_send` и `ch10_raw_send` лежат в одной секции 54 с одним
адресом и размером: LLVM получил для них одинаковый код и оставил одну
функцию, а второе имя сделал псевдонимом (проход MergeFunctions из
раздела [10.5](05-mono.md)). Typestate не стоил ни одной инструкции.
Тест `rustc_typestate_is_free` в `code/ch10/tests/cost.rs` проверяет
именно это совпадение. Сама функция — пять записей настройки, байт в
DR, ожидание BUSY и выключение:

```console
$ cargo asm -p ch10 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch10_raw_send
ch10_raw_send:
	push {r4, r5, r7, lr}
	add r7, sp, #8
	movs r5, #0
	mov r4, r0
	str r5, [r0, #48]
	movs r0, #6
	str r0, [r4, #36]
	movs r0, #33
	str r0, [r4, #40]
	movs r0, #112
	str r0, [r4, #44]
	movw r0, #769
	str r0, [r4, #48]
	mov r0, r4
	bl <ch06_regs::Pl011>::write_byte
…
```

CR = 0 по смещению 48, IBRD = 6 по 36, FBRD = 33 по 40, LCRH = 0x70 по
44, CR = 0x301, затем байт и цикл опроса FR. Флаг во время работы стоит
дороже:

```console
$ cargo asm -p ch10 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch10_checked_write
ch10_checked_write:
	push {r4, r6, r7, lr}
	add r7, sp, #8
	ldrb r4, [r0, #4]
	cmp r4, #1
	itt eq
	ldreq r0, [r0]
	bleq <ch06_regs::Pl011>::write_byte
	mov r0, r4
	pop {r4, r6, r7, pc}
$ cargo asm -p ch10 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch10_typed_write
ch10_typed_write:
	push {r7, lr}
	mov r7, sp
	ldr r0, [r0]
	pop.w {r7, lr}
	b <ch06_regs::Pl011>::write_byte
```

`UartChecked` занимает 8 байт вместо 4, каждая запись загружает флаг и
сравнивает его, а результат `Result` приходится проверять вызывающему.
Главное же не в байтах: ошибка «пишем в выключенный UART» в варианте с
флагом обнаруживается во время работы, и решать, что с ней делать на
устройстве без экрана, придётся тоже во время работы. У typestate её нет.

## В QEMU

Прошивка `firmware/ch10-typestate` проводит UART0 машины `lm3s6965evb`
через оба состояния:

```rust
{{#include ../../firmware/ch10-typestate/src/main.rs:main}}
```

```console
$ cd firmware && cargo run --release -q -p ch10-typestate
Timer with period zero, disabling
Uart<Enabled>: 4 байта, Uart<Disabled>: 4 байта, &Pl011: 4 байта
IBRD = 6, FBRD = 33, CR = 0x301
после disable: CR = 0x0
снова включён: CR = 0x301
```

Строка про таймер — от самого QEMU, как в разделе
[0.6](../ch00/06-qemu.md). Обе строки после `disable` проверяет
`firmware/run-demos.sh`. Сообщение о выключенном UART идёт через
semihosting: у `Uart<Disabled>` нет метода, которым его можно было бы
напечатать.

<div class="box trap">

**Ловушка.** Модель PL011 в QEMU 8.2 проще настоящей микросхемы. Она не
проверяет бит UARTEN: байт, записанный в DR выключенного UART, всё равно
появляется в выводе. И она никогда не поднимает BUSY: FR после записи
читается как 0x90, «буферы пусты», поэтому цикл ожидания в `disable`
завершается сразу. На кристалле при UARTEN = 0 передатчик стоит, и байт
в линию не уходит, а `disable` ждёт, пока передатчик опустошит FIFO: до
16 байт по 87 мкс при 115 200 бод. Прошивка, которая «работает в QEMU», может не работать на
плате, и typestate закрывает как раз ту ошибку, которую эмулятор
пропустил бы.

</div>

<div class="box hood">

**Под капотом.** Typestate не защищает от второго драйвера на тех же
регистрах: `Pl011::at` — `unsafe`, и ничто не мешает вызвать его дважды
и получить два `Uart<Disabled>`. Что регистры принадлежат одному
владельцу, обеспечивают синглтоны периферии: функция `take()`, которая
отдаёт структуру с регистрами один раз за время работы. Так устроены
крейты доступа к периферии, их разбирает глава 27.

</div>

## Когда typestate не подходит

Состояние в типе известно при компиляции. Если оно определяется во
время работы — драйвер лежит в поле структуры, которая живёт дольше
одной функции, или состояние приходит от железа, — тип поля один, и
приходится заворачивать варианты в перечисление: `enum AnyUart {
Off(Uart<Disabled>), On(Uart<Enabled>) }`. Проверка возвращается во
время работы, только в виде `match`. Второе неудобство — ошибки: если
переход может не удаться, драйвер должен вернуться вызывающему вместе с
ошибкой, иначе неудачный `enable` уничтожит его вместе с шиной. Оба
приёма понадобятся в задаче D.

<div class="box check">

**Проверь себя.** Что будет, если поле `_state` сделать публичным?
Почему `Addr<Phys>` и `Addr<Virt>` нельзя сравнить через `==`, хотя
внутри у обоих `usize`, и как это разрешить, не разрешив путать их?
Сколько байт займёт `AnyUart` из последнего абзаца на Cortex-M3?

</div>
