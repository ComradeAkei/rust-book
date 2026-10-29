# 27.3. PAC из описания кристалла

## SVD

Регистры периферии кристалла описаны в его техническом описании —
сотнях страниц таблиц. Те же таблицы производитель выпускает в машинном
виде: файл *CMSIS-SVD* (System View Description), XML со всеми блоками,
регистрами, полями, значениями после сброса и номерами прерываний. Из SVD
строят заголовки C для CMSIS, по нему отладчики показывают регистры по
именам, и из него же svd2rust строит **PAC** (peripheral access crate) —
крейт доступа к периферии.

В книге SVD — подмножество LM3S6965: системное управление, семь портов
GPIO, два UART, четыре таймера и АЦП, — написанное по техническому
описанию, в `code/ch27/svd/lm3s6965.svd`. Начинается оно с ядра:

```xml
{{#include ../../code/ch27/svd/lm3s6965.svd:cpu}}
```

`nvicPrioBits` — те три бита приоритета из раздела [27.2](02-nvic.md).
Дальше — блоки. UART0 — с адреса 0x4000_C000, его прерывание — номер 5,
и первый регистр — DR:

```xml
{{#include ../../code/ch27/svd/lm3s6965.svd:uart0}}
```

Поле DATA — восемь битов с ограничением записи «любое число от 0 до
255». Поле FE — флаг ошибки кадра, только для чтения: так пишет
PL011 в DR, когда принятый байт пришёл без стопового бита. А у регистра
порогов FIFO ограничений нет:

```xml
{{#include ../../code/ch27/svd/lm3s6965.svd:ifls}}
```

Три бита TXIFLSEL по техническому описанию принимают значения от 0 до
4, а 5–7 зарезервированы. В SVD это не записано, и разница скажется на
API.

## svd2rust

PAC строит скрипт:

```bash
{{#include ../../code/ch27/svd/gen-pac.sh}}
```

svd2rust выдаёт три файла: `lib.rs` — весь крейт, около 7000 строк
после rustfmt; `device.x` — фрагмент [скрипта компоновки][g-linker-script]; `build.rs`,
который кладёт `device.x` туда, где его найдёт компоновщик. Руками их не
правят: шаг CI «PAC главы 27 совпадает с выводом svd2rust» запускает
скрипт с закреплённой версией svd2rust и проверяет `git diff
--exit-code code/ch27-pac`. Правка SVD без перегенерации, правка PAC
мимо SVD и смена версии генератора ломают сборку.

Каждый блок периферии — тип нулевого размера с адресом в параметре:

```rust,ignore
pub type Uart0 = crate::Periph<uart0::RegisterBlock, 0x4000_c000>;

pub struct Periph<RB, const A: usize> {
    _marker: marker::PhantomData<RB>,
}
impl<RB, const A: usize> core::ops::Deref for Periph<RB, A> {
    type Target = RB;
    fn deref(&self) -> &Self::Target {
        unsafe { &*Self::PTR }
    }
}
```

`Uart0` не хранит указатель: адрес известен при компиляции, и `Deref`
превращает его в `&RegisterBlock`. Сам `RegisterBlock` — `#[repr(C)]`
структура, у которой поля стоят по смещениям регистров, а промежутки
заполнены массивами `_reserved`. Регистр — `Reg<DrSpec>` над
`VolatileCell<u32>`: каждое обращение — `read_volatile` или
`write_volatile`, как в разделе [17.5](../ch17/05-volatile.md). Это та
же конструкция, что блок регистров PL011 из раздела
[6.9](../ch06/09-registers.md), только написанная не руками, а
генератором — для каждого регистра каждого блока.

## read, write, modify

Регистр — слово по определённому адресу, а поле — несколько его битов
с отдельным смыслом. Например, один бит сообщает «передатчик занят»,
другой включает устройство. Цепочка методов PAC — способ назвать
нужное поле вместо того, чтобы вручную считать маску и сдвиг.
Она не создаёт копию устройства: конечное чтение или запись всё ещё
обращается к тому же аппаратному адресу.

У каждого регистра — до трёх операций, и каждая — одно обращение к
шине или два:

- `read()` — одно чтение. Возвращает `R`, снимок значения, с методом на
  каждое поле: `uart.fr().read().txff().bit_is_set()`;
- `write(|w| …)` — одна запись. [Замыкание][g-closure] получает `W` и
  устанавливает поля; что не установлено, остаётся *значением после
  сброса* из SVD, а не нулём;
- `modify(|r, w| …)` — чтение, замыкание, запись. Замыкание видит
  старое значение в `r` и меняет в `w` только нужные поля.

Драйвер UART0 из раздела [27.5](05-hal.md) настраивает регистры
записью:

```rust
{{#include ../../code/ch27-hal/src/serial.rs:new}}
```

А включение тактов блока — изменение: в RCGC1 биты всех UART и таймеров,
и чужие трогать нельзя.

```rust
{{#include ../../code/ch27-hal/src/sysctl.rs:enable}}
```

`w.uart(n)` — поле из массива: в SVD имя `UART%s` с `dim`, и svd2rust
сделал из трёх одинаковых полей, UART0–UART2, один метод с номером.

<div class="box trap">

**Ловушка.** `write` начинает со значения после сброса, а не с нуля. У
CTL оно 0x0300: передатчик и приёмник включены. `ctl().write(|w|
w.uarten().set_bit())` даст 0x0301, а не 0x0001, как
`UART0->CTL = UARTEN;` в C. Чтобы начать с нуля, есть `write_with_zero`,
и она `unsafe`: ноль может быть недопустимым значением регистра. А
`modify` — три шага, и прерывание между чтением и записью, которое
меняет тот же регистр, своё изменение потеряет: `modify` не атомарен
(раздел [27.6](06-interrupts.md)).

</div>

## Безопасные и небезопасные поля

Ограничения записи из SVD svd2rust переводит в типы. Поле DATA
принимает любое значение своей ширины — его метод `set` безопасен. У
TXIFLSEL ограничения нет, и svd2rust не знает, что 5 — плохое значение;
поэтому безопасного `set` у поля нет, есть только `unsafe fn bits`.
Запись регистра целиком — тоже `unsafe bits`. Поля только для чтения в
`W` нет совсем. Поле с перечислением значений получает метод на каждое:
у OSCSRC регистра RCC — `main()`, `internal()` и `internal_div4()`.

Это проверяют пробы, собранные против PAC:

```rust
{{#include ../../code/ch27-hal/tests/typestate.rs:pac}}
```

```text
error[E0133]: call to unsafe function `…FieldWriter<'a, REG, WI, FI, Safety>>::bits` is unsafe and requires unsafe function or block
error[E0599]: no method named `set` found for struct `…FieldWriter<'_, IflsSpec, 3>` in the current scope
error[E0133]: call to unsafe function `…W<REG>>::bits` is unsafe and requires unsafe function or block
error[E0599]: no method named `fe` found for mutable reference `&mut …W<DrSpec>` in the current scope
```

`unsafe` здесь — не про память программы, а про железо: запись значения,
которого нет в техническом описании, может перевести блок в состояние,
которого нет в документации. Насколько безопасен PAC, настолько точен
SVD. Описания производителей бывают неполными, и для популярных
кристаллов сообщество Rust поддерживает исправленные: ограничения,
перечисления и описания в YAML-файлах поверх оригинального SVD (проект
stm32-rs и подобные).

## Прерывания по именам

Из блоков `<interrupt>` svd2rust строит три вещи. Перечисление:

```rust,ignore
pub enum Interrupt {
    GPIOA = 0,
    GPIOB = 1,
    …
}
```

— его и принимают `NVIC::pend`, `NVIC::unmask` и `set_priority` в
разделе [27.2](02-nvic.md): номер прерывания не может быть числом с
опечаткой. Таблицу векторов прерываний `__INTERRUPTS` — массив из 37
адресов обработчиков, по одному на номер до последнего описанного, с
нулями на месте номеров, которых в SVD нет. И `device.x`:

```text
PROVIDE(GPIOA = DefaultHandler);
PROVIDE(GPIOB = DefaultHandler);
…
```

`PROVIDE` — слабое определение символа в скрипте компоновки: если в
программе нет функции `GPIOA`, символ указывает на `DefaultHandler`.
cortex-m-rt [с возможностью][g-feature] `device` включает `device.x` в свой `link.x`.

Обработчик объявляет атрибут `#[interrupt]`, и имя функции он сверяет с
перечислением. Прошивка `ch27-irq` с возможностью `typo` объявляет
обработчик с опечаткой:

```rust
{{#include ../../firmware/ch27-irq/src/main.rs:typo}}
```

```console
$ cd firmware && cargo build --release -p ch27-irq --features typo
error[E0599]: no variant, associated function, or constant named `TIMER0X` found for enum `ch27_hal::ch27_pac::Interrupt` in the current scope
  --> ch27-irq/src/main.rs:65:4
   |
65 | fn TIMER0X() {}
   |    ^^^^^^^ variant, associated function, or constant not found in `ch27_hal::ch27_pac::Interrupt`
   |
help: there is a variant with a similar name
   |
65 - fn TIMER0X() {}
65 + fn TIMER0A() {}
```

В C `void TIMER0X_Handler(void)` собралась бы молча: слабый
`TIMER0A_Handler` из файла запуска остался бы `Default_Handler`, а
новая функция — просто функцией, которую никто не зовёт. `run-demos.sh`
проверяет, что такая сборка падает с этим текстом.

<div class="box c">

**Как в C.** Заголовок CMSIS из того же SVD (утилита SVDConv):
`typedef struct { __IO uint32_t DR; … } UART0_Type;` и
`#define UART0 ((UART0_Type *)0x4000C000)`. Регистры — `volatile`
поля, поля — макросы `UART_CTL_UARTEN` и маски, запись — `|=` и `&= ~`.
PAC даёт то же расположение в памяти и те же обращения к шине, но поля
— методы с проверенной шириной, `modify` не забудет маску, а `write`
не тронет поле только для чтения, потому что его в `W` нет.

</div>

<div class="box hood">

**Под капотом.** Флаги, которые сбрасываются записью единицы, в SVD
помечают `modifiedWriteValues = oneToClear`. Для таких полей svd2rust
в `write` и `modify` начинает с нуля в их битах: иначе `modify`,
прочитав взведённый флаг, записал бы его единицей обратно и сбросил,
хотя менялось другое поле. В книжном SVD таких пометок нет: регистры
ICR, где они нужны, драйверы пишут только `write`.

</div>

<div class="box check">

**Проверь себя.** Что запишет `uart.ifls().write(|w| unsafe {
w.txiflsel().bits(0) })` в RXIFLSEL? Почему `modify` на регистре ICR —
ошибка, даже если бы он был безопасен по типам? Что нужно добавить в
SVD, чтобы `set` появился у TXIFLSEL и [паниковал][g-panic] на значении 5?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-closure]: ../appendix-f.md#closure "Замыкание (closure): Функция вместе со своим контекстом — захваченными переменными."
[g-feature]: ../appendix-f.md#feature "Возможность (feature): Именованный переключатель пакета Cargo, который превращается в условие feature для #[cfg]."
[g-linker-script]: ../appendix-f.md#linker-script "Скрипт компоновщика (linker script): Файл, который говорит компоновщику, по каким адресам разместить секции программы: таблицу векторов — по адресу 0, код и константы — во флеш-память, .data и .bss — в ОЗУ."
[g-panic]: ../appendix-f.md#panic "Паника (panic): Аварийная реакция на ошибку в самой программе: индекс за границей, unwrap на None, переполнение в отладочной сборке."
