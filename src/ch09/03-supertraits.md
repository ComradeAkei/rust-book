# 9.3. Супертрейты

## Требование, а не наследование

Опрос устройства на шине I2C должен уметь две вещи сверх самого
опроса: назвать устройство и напечатать его состояние для журнала.
Трейт может потребовать это от реализаций:

```rust
{{#include ../../code/ch09/src/supertrait.rs:supertraits}}
```

Запись `trait Probe: Named + fmt::Debug` читается как «реализовать
`Probe` может только тип, который уже реализует `Named` и `Debug`».
`Named` и `Debug` здесь — *супертрейты* (supertraits) `Probe`. Метод по
умолчанию `report` пользуется ими как своими: `self.name()` пришёл из
`Named`, а `{self:?}` — из `Debug`. Он вправе это делать именно потому,
что у любого `Self` эти реализации гарантированно есть.

Реализации супертрейтов пишутся отдельными блоками, каждая для своего
трейта:

```rust
{{#include ../../code/ch09/src/supertrait.rs:impls}}
```

```text
BMP280: id 0x58
BMP280: NoAck, состояние Bmp { addr: 119 }
24C02: идентификатора нет, это память
```

Первые две строки напечатал метод по умолчанию для датчика по адресам
0x76 и 0x77, третью — собственный `report` памяти `Eeprom`. Тип без
`Named` реализовать `Probe` не может:

```text
error[E0277]: the trait bound `Nameless: ch09::supertrait::Named` is not satisfied
 --> e0277sup.rs:4:16
  |
4 | impl Probe for Nameless {
  |                ^^^^^^^^ unsatisfied trait bound
  |
help: the trait `ch09::supertrait::Named` is not implemented for `Nameless`
…
note: required by a bound in `Probe`
```

## Супертрейты в ограничениях

Ограничение `T: Probe` в обобщённой функции открывает и методы
супертрейтов:

```rust
{{#include ../../code/ch09/src/supertrait.rs:bound}}
```

Писать `T: Probe + Named` не нужно: из `T: Probe` компилятор сам выводит
`T: Named` и `T: Debug`. Обратное не верно: функция с ограничением
`T: Named` не может вызвать `probe`, ведь не всё, что умеет называться,
умеет опрашиваться.

<div class="box c">

**Как в C.** «Наследование» в C — структура базового типа первым полем
производного и приведение указателей: `struct bmp { struct device dev;
… }`, а `(struct device *)&bmp` даёт базовую часть. Это вопрос
раскладки памяти, и он работает, пока кто-нибудь не переставит поля.
Супертрейт к раскладке отношения не имеет: у трейтов нет полей, а
требование `Probe: Named` говорит только о том, какие ещё реализации
должны быть у типа. Данные `Bmp` описывает сам `Bmp`.

</div>

<div class="box trap">

**Ловушка.** Методы супертрейта нельзя реализовать в блоке подтрейта, как
переопределить метод базового класса:

```rust
impl Probe for Mpu {
    fn probe(&mut self) -> Result<u8, ProbeError> { Ok(0x68) }
    fn name(&self) -> &'static str { "MPU6050" }
}
```

```text
error[E0407]: method `name` is not a member of trait `Probe`
error[E0277]: the trait bound `Mpu: ch09::supertrait::Named` is not satisfied
```

`name` принадлежит `Named`, и написать его можно только в `impl Named for
Mpu`. Обе реализации — независимые блоки, и порядок между ними не важен.

</div>

<div class="box hood">

**Под капотом.** Параметр `out` у `report` имеет тип `&mut dyn
fmt::Write`: туда подойдёт и `String` в тесте, и драйвер [UART][g-uart] в
прошивке, если он реализует `fmt::Write`. Тогда `report` пишет прямо в
порт, без промежуточного буфера в куче. Что такое `dyn` и чем он платит
за гибкость, разбирает глава 11.

</div>

<div class="box check">

**Проверь себя.** Почему метод по умолчанию в `Probe` может печатать
`{self:?}`, а метод по умолчанию в `Sensor` из раздела
[9.1](01-traits.md) — нет? Что придётся изменить в `Probe`, если
потребовать от устройств ещё и `Clone`, и каким типам из этого раздела
придётся добавить реализацию?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-uart]: ../appendix-f.md#uart "UART (Universal Asynchronous Receiver-Transmitter): Последовательный порт: передаёт байты по линии бит за битом без общего тактового сигнала."
